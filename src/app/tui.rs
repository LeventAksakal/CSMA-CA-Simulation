use std::time::{Duration, Instant};

use anyhow::{Result, ensure};
use clap::ValueEnum;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, Wrap},
};

use crate::{
    Scenario, TimingConfig,
    sim::{SimulationTrace, TraceFrame, trace},
};

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum DemoPreset {
    Single,
    Collision,
    Mixed,
}

pub fn run_demo(preset: DemoPreset, seed: u64, slots: u64, tick_ms: u64) -> Result<()> {
    ensure!(tick_ms > 0, "tick_ms must be greater than zero");

    let scenario = build_demo_scenario(preset, seed, slots);
    let trace = trace(&scenario)?;
    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal, DemoApp::new(preset, seed, tick_ms, trace));
    ratatui::restore();
    result
}

fn run_app(terminal: &mut ratatui::DefaultTerminal, mut app: DemoApp) -> Result<()> {
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|frame| app.render(frame))?;

        let timeout = if app.paused {
            Duration::from_millis(100)
        } else {
            app.tick_rate.saturating_sub(last_tick.elapsed())
        };

        if event::poll(timeout)? {
            let event = event::read()?;
            if app.handle_event(event)? {
                break;
            }
        }

        if !app.paused && last_tick.elapsed() >= app.tick_rate {
            app.advance();
            last_tick = Instant::now();
        }
    }

    Ok(())
}

fn build_demo_scenario(preset: DemoPreset, seed: u64, slots: u64) -> Scenario {
    match preset {
        DemoPreset::Single => Scenario::standard(
            1,
            8,
            seed,
            TimingConfig {
                total_slots: slots,
                payload_bits: 12_000,
                difs_slots: 1,
                sifs_slots: 0,
                tx_duration_slots: 1,
            },
            32,
        ),
        DemoPreset::Collision => Scenario::standard(
            6,
            1,
            seed,
            TimingConfig {
                total_slots: slots,
                payload_bits: 12_000,
                difs_slots: 1,
                sifs_slots: 0,
                tx_duration_slots: 1,
            },
            31,
        ),
        DemoPreset::Mixed => Scenario::mixed(
            5,
            5,
            4,
            16,
            seed,
            TimingConfig {
                total_slots: slots,
                payload_bits: 12_000,
                difs_slots: 1,
                sifs_slots: 0,
                tx_duration_slots: 1,
            },
            63,
        ),
    }
}

struct DemoApp {
    preset: DemoPreset,
    seed: u64,
    tick_rate: Duration,
    paused: bool,
    frame_index: usize,
    trace: SimulationTrace,
}

impl DemoApp {
    fn new(preset: DemoPreset, seed: u64, tick_ms: u64, trace: SimulationTrace) -> Self {
        Self {
            preset,
            seed,
            tick_rate: Duration::from_millis(tick_ms),
            paused: false,
            frame_index: 0,
            trace,
        }
    }

    fn render(&self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(18),
                Constraint::Length(9),
            ])
            .split(frame.area());
        let main = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(66), Constraint::Percentage(34)])
            .split(layout[1]);
        let side = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Min(6),
            ])
            .split(main[1]);

        self.render_header(frame, layout[0]);
        self.render_station_table(frame, main[0]);
        self.render_summary(frame, side[0]);
        self.render_class_table(frame, side[1]);
        self.render_controls(frame, side[2]);
        self.render_event_log(frame, layout[2]);
    }

    fn handle_event(&mut self, event: Event) -> Result<bool> {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return Ok(false);
            }

            match key.code {
                KeyCode::Char('q') => return Ok(true),
                KeyCode::Char(' ') => self.paused = !self.paused,
                KeyCode::Char('n') => {
                    self.paused = true;
                    self.advance();
                }
                KeyCode::Char('r') => {
                    self.frame_index = 0;
                    self.paused = false;
                }
                KeyCode::Char('f') => {
                    let millis = self.tick_rate.as_millis() as u64;
                    self.tick_rate = Duration::from_millis((millis / 2).max(25));
                }
                KeyCode::Char('s') => {
                    let millis = self.tick_rate.as_millis() as u64;
                    self.tick_rate = Duration::from_millis((millis + 25).min(2_000));
                }
                _ => {}
            }
        }

        Ok(false)
    }

    fn advance(&mut self) {
        if self.frame_index + 1 < self.trace.frames.len() {
            self.frame_index += 1;
        } else {
            self.paused = true;
        }
    }

    fn current_frame(&self) -> &TraceFrame {
        &self.trace.frames[self.frame_index]
    }

    fn recent_events(&self) -> Vec<String> {
        let start = self.frame_index.saturating_sub(7);
        self.trace.frames[start..=self.frame_index]
            .iter()
            .map(|frame| format!("slot {:>3}: {}", frame.slot, event_label(frame)))
            .collect()
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let current = self.current_frame();
        let status = if self.paused { "paused" } else { "running" };
        let medium = if current.medium_busy {
            format!("busy ({})", current.medium_busy_slots_remaining)
        } else {
            format!("idle ({})", current.idle_slots)
        };
        let text = Line::from(vec![
            Span::styled(
                "DCF Demo ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "preset={} seed={} slot={}/{} medium={} speed={}ms status={}",
                preset_label(self.preset),
                self.seed,
                current.slot + 1,
                self.trace.frames.len(),
                medium,
                self.tick_rate.as_millis(),
                status,
            )),
        ]);

        frame.render_widget(
            Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Live Trace")),
            area,
        );
    }

    fn render_station_table(&self, frame: &mut Frame, area: Rect) {
        let rows = self.current_frame().stations.iter().map(|station| {
            Row::new(vec![
                Cell::from(station.id.to_string()),
                Cell::from(station.class_name.clone()),
                Cell::from(phase_label(station.phase)),
                Cell::from(station.backoff_counter.to_string()),
                Cell::from(
                    station
                        .frozen_backoff_counter
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| String::from("-")),
                ),
                Cell::from(station.current_cw.to_string()),
                Cell::from(station.defer_slots_remaining.to_string()),
                Cell::from(station.successful_packets.to_string()),
                Cell::from(station.collision_attempts.to_string()),
            ])
            .style(phase_style(station.phase))
        });
        let widths = [
            Constraint::Length(4),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(6),
            Constraint::Length(7),
            Constraint::Length(6),
            Constraint::Length(10),
        ];

        let table = Table::new(rows, widths)
            .header(
                Row::new(vec![
                    "id",
                    "class",
                    "phase",
                    "backoff",
                    "frozen",
                    "cw",
                    "defer",
                    "sent",
                    "collisions",
                ])
                .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .block(Block::default().borders(Borders::ALL).title("Stations"));

        frame.render_widget(table, area);
    }

    fn render_summary(&self, frame: &mut Frame, area: Rect) {
        let current = self.current_frame();
        let aggregate = &current.progress.aggregate;
        let lines = vec![
            Line::from(format!("event: {}", event_label(current))),
            Line::from(format!(
                "successful packets: {}",
                aggregate.total_successful_packets
            )),
            Line::from(format!("collision events: {}", aggregate.collision_events)),
            Line::from(format!(
                "avg delay: {:.2} slots",
                aggregate.average_delay_slots
            )),
            Line::from(format!(
                "throughput: {:.2} bits/slot",
                aggregate.throughput_bits_per_slot
            )),
            Line::from(format!("elapsed slots: {}", current.progress.elapsed_slots)),
        ];

        frame.render_widget(
            Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title("Summary"))
                .wrap(Wrap { trim: true }),
            area,
        );
    }

    fn render_class_table(&self, frame: &mut Frame, area: Rect) {
        let rows = self.current_frame().progress.per_class.iter().map(|class| {
            Row::new(vec![
                Cell::from(class.class_name.clone()),
                Cell::from(class.users.to_string()),
                Cell::from(class.successful_packets.to_string()),
                Cell::from(class.collision_attempts.to_string()),
                Cell::from(format!("{:.1}", class.average_delay_slots)),
                Cell::from(format!("{:.1}", class.throughput_bits_per_slot)),
            ])
        });

        let table = Table::new(
            rows,
            [
                Constraint::Length(12),
                Constraint::Length(5),
                Constraint::Length(6),
                Constraint::Length(10),
                Constraint::Length(8),
                Constraint::Length(9),
            ],
        )
        .header(
            Row::new(vec![
                "class",
                "users",
                "sent",
                "collisions",
                "delay",
                "throughput",
            ])
            .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(Block::default().borders(Borders::ALL).title("Per Class"));

        frame.render_widget(table, area);
    }

    fn render_controls(&self, frame: &mut Frame, area: Rect) {
        let lines = vec![
            Line::from("space: pause/resume"),
            Line::from("n: step one slot"),
            Line::from("f: faster playback"),
            Line::from("s: slower playback"),
            Line::from("r: restart trace"),
            Line::from("q: quit"),
        ];

        frame.render_widget(
            Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title("Controls"))
                .wrap(Wrap { trim: true }),
            area,
        );
    }

    fn render_event_log(&self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .recent_events()
            .into_iter()
            .map(ListItem::new)
            .collect();

        frame.render_widget(
            List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Recent Events"),
            ),
            area,
        );
    }
}

fn event_label(frame: &TraceFrame) -> String {
    match &frame.event {
        crate::sim::dcf::engine::SlotEvent::Busy {
            busy_slots_remaining,
        } => format!("medium busy, {} busy slots remain", busy_slots_remaining),
        crate::sim::dcf::engine::SlotEvent::Idle => String::from("idle contention slot"),
        crate::sim::dcf::engine::SlotEvent::Success { station_id } => {
            format!("station {} transmitted successfully", station_id)
        }
        crate::sim::dcf::engine::SlotEvent::Collision { station_ids } => {
            format!("collision between stations {:?}", station_ids)
        }
    }
}

fn phase_label(phase: crate::sim::dcf::phase::StationPhase) -> &'static str {
    match phase {
        crate::sim::dcf::phase::StationPhase::WaitingForMedium => "waiting",
        crate::sim::dcf::phase::StationPhase::Defer => "defer",
        crate::sim::dcf::phase::StationPhase::BackoffCountdown => "backoff",
        crate::sim::dcf::phase::StationPhase::Transmitting => "tx",
        crate::sim::dcf::phase::StationPhase::AwaitingResult => "awaiting",
        crate::sim::dcf::phase::StationPhase::CollisionRecovery => "recovery",
    }
}

fn phase_style(phase: crate::sim::dcf::phase::StationPhase) -> Style {
    match phase {
        crate::sim::dcf::phase::StationPhase::WaitingForMedium => Style::default().fg(Color::Gray),
        crate::sim::dcf::phase::StationPhase::Defer => Style::default().fg(Color::Yellow),
        crate::sim::dcf::phase::StationPhase::BackoffCountdown => Style::default().fg(Color::Cyan),
        crate::sim::dcf::phase::StationPhase::Transmitting => Style::default().fg(Color::Green),
        crate::sim::dcf::phase::StationPhase::AwaitingResult => Style::default().fg(Color::Blue),
        crate::sim::dcf::phase::StationPhase::CollisionRecovery => {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        }
    }
}

fn preset_label(preset: DemoPreset) -> &'static str {
    match preset {
        DemoPreset::Single => "single",
        DemoPreset::Collision => "collision",
        DemoPreset::Mixed => "mixed",
    }
}

#[cfg(test)]
mod tests {
    use super::{DemoPreset, build_demo_scenario};

    #[test]
    fn mixed_preset_creates_two_classes() {
        let scenario = build_demo_scenario(DemoPreset::Mixed, 7, 120);

        assert_eq!(scenario.classes.len(), 2);
        assert_eq!(scenario.classes[0].name, "lower-cw");
        assert_eq!(scenario.classes[1].name, "higher-cw");
    }

    #[test]
    fn collision_preset_uses_high_contention() {
        let scenario = build_demo_scenario(DemoPreset::Collision, 7, 120);

        assert_eq!(scenario.classes.len(), 1);
        assert_eq!(scenario.classes[0].cw_min, 1);
        assert_eq!(scenario.classes[0].users, 6);
    }
}
