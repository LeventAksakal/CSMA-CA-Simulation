use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, ensure};
use clap::ValueEnum;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Sparkline, Table, Wrap},
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemoOptions {
    pub preset: DemoPreset,
    pub seed: u64,
    pub slots: u64,
    pub tick_ms: u64,
    pub replay: Option<PathBuf>,
    pub export_trace: Option<PathBuf>,
    pub compare_seed: Option<u64>,
    pub compare_cw_min: Option<u32>,
    pub compare_replay: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct DemoPane {
    label: String,
    trace: SimulationTrace,
}

pub fn run_demo(options: DemoOptions) -> Result<()> {
    ensure!(options.tick_ms > 0, "tick_ms must be greater than zero");

    let panes = build_demo_panes(&options)?;
    if let Some(path) = &options.export_trace {
        write_trace(path, &panes[0].trace)?;
    }

    let mut terminal = ratatui::init();
    let result = run_app(
        &mut terminal,
        DemoApp::new(Duration::from_millis(options.tick_ms), panes),
    );
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

fn build_demo_panes(options: &DemoOptions) -> Result<Vec<DemoPane>> {
    ensure!(options.slots > 0, "slots must be greater than zero");
    if options.replay.is_some() {
        ensure!(
            options.compare_seed.is_none() && options.compare_cw_min.is_none(),
            "compare-seed and compare-cw-min require a generated primary scenario, not replay"
        );
    }

    let primary = if let Some(path) = &options.replay {
        DemoPane {
            label: format!("replay {}", path.display()),
            trace: read_trace(path)?,
        }
    } else {
        let scenario = build_demo_scenario(options.preset, options.seed, options.slots);
        DemoPane {
            label: format!(
                "{} seed={} cw={} slots={}",
                preset_label(options.preset),
                options.seed,
                cw_label(&scenario),
                scenario.timing.total_slots,
            ),
            trace: trace(&scenario)?,
        }
    };

    let compare = if let Some(path) = &options.compare_replay {
        Some(DemoPane {
            label: format!("compare replay {}", path.display()),
            trace: read_trace(path)?,
        })
    } else if options.compare_seed.is_some() || options.compare_cw_min.is_some() {
        let base = build_demo_scenario(options.preset, options.seed, options.slots);
        let scenario = build_compare_scenario(&base, options.compare_seed, options.compare_cw_min)?;
        Some(DemoPane {
            label: format!("compare seed={} cw={}", scenario.seed, cw_label(&scenario)),
            trace: trace(&scenario)?,
        })
    } else {
        None
    };

    let mut panes = vec![primary];
    if let Some(compare) = compare {
        panes.push(compare);
    }

    Ok(panes)
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
                collision_penalty_slots: 4,
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
                collision_penalty_slots: 4,
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
                collision_penalty_slots: 4,
            },
            63,
        ),
    }
}

fn build_compare_scenario(
    base: &Scenario,
    compare_seed: Option<u64>,
    compare_cw_min: Option<u32>,
) -> Result<Scenario> {
    let mut scenario = base.clone();

    if let Some(seed) = compare_seed {
        scenario.seed = seed;
    }

    if let Some(compare_cw_min) = compare_cw_min {
        ensure!(
            compare_cw_min > 0,
            "compare-cw-min must be greater than zero"
        );

        let base_cw = scenario
            .classes
            .first()
            .map(|class| class.cw_min)
            .unwrap_or(compare_cw_min)
            .max(1);
        for class in &mut scenario.classes {
            let scaled = (u64::from(class.cw_min) * u64::from(compare_cw_min)
                + (u64::from(base_cw) / 2))
                / u64::from(base_cw);
            class.cw_min = (scaled as u32).max(1).min(scenario.window.cw_max);
        }
    }

    Ok(scenario)
}

fn write_trace(path: &Path, trace: &SimulationTrace) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let content = serde_json::to_string_pretty(trace)
        .with_context(|| format!("failed to serialize trace for {}", path.display()))?;
    fs::write(path, content).with_context(|| format!("failed to write {}", path.display()))
}

fn read_trace(path: &Path) -> Result<SimulationTrace> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read replay trace {}", path.display()))?;
    let trace: SimulationTrace = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse replay trace {}", path.display()))?;
    ensure!(
        !trace.frames.is_empty(),
        "{} did not contain any frames",
        path.display()
    );
    Ok(trace)
}

struct DemoApp {
    tick_rate: Duration,
    paused: bool,
    frame_index: usize,
    teaching_mode: bool,
    panes: Vec<DemoPane>,
}

impl DemoApp {
    fn new(tick_rate: Duration, panes: Vec<DemoPane>) -> Self {
        Self {
            tick_rate,
            paused: false,
            frame_index: 0,
            teaching_mode: true,
            panes,
        }
    }

    fn render(&self, frame: &mut Frame) {
        let root = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(24),
                Constraint::Length(7),
                Constraint::Length(9),
            ])
            .split(frame.area());

        self.render_header(frame, root[0]);
        self.render_panes(frame, root[1]);
        self.render_teaching(frame, root[2]);
        self.render_footer(frame, root[3]);
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
                KeyCode::Char('t') => self.teaching_mode = !self.teaching_mode,
                _ => {}
            }
        }

        Ok(false)
    }

    fn advance(&mut self) {
        if self.frame_index + 1 < self.max_frames() {
            self.frame_index += 1;
        } else {
            self.paused = true;
        }
    }

    fn max_frames(&self) -> usize {
        self.panes
            .iter()
            .map(|pane| pane.trace.frames.len())
            .max()
            .unwrap_or(1)
    }

    fn current_frame(&self, pane_index: usize) -> &TraceFrame {
        let pane = &self.panes[pane_index];
        let index = self
            .frame_index
            .min(pane.trace.frames.len().saturating_sub(1));
        &pane.trace.frames[index]
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let status = if self.paused { "paused" } else { "running" };
        let compare = if self.panes.len() > 1 {
            "compare=on"
        } else {
            "compare=off"
        };
        let teaching = if self.teaching_mode {
            "teaching=on"
        } else {
            "teaching=off"
        };
        let text = Line::from(vec![
            Span::styled(
                "DCF Demo ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "slot={}/{} speed={}ms status={} {} {}",
                self.frame_index + 1,
                self.max_frames(),
                self.tick_rate.as_millis(),
                status,
                compare,
                teaching,
            )),
        ]);

        frame.render_widget(
            Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Live Trace")),
            area,
        );
    }

    fn render_panes(&self, frame: &mut Frame, area: Rect) {
        let areas = if self.panes.len() > 1 {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(100)])
                .split(area)
        };

        for (index, pane_area) in areas.iter().enumerate() {
            self.render_pane(frame, *pane_area, index);
        }
    }

    fn render_pane(&self, frame: &mut Frame, area: Rect, pane_index: usize) {
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Length(7),
                Constraint::Min(8),
            ])
            .split(area);

        self.render_summary(frame, sections[0], pane_index);
        self.render_sparklines(frame, sections[1], pane_index);
        self.render_class_table(frame, sections[2], pane_index);
        self.render_station_table(frame, sections[3], pane_index);
    }

    fn render_summary(&self, frame: &mut Frame, area: Rect, pane_index: usize) {
        let pane = &self.panes[pane_index];
        let current = self.current_frame(pane_index);
        let aggregate = &current.progress.aggregate;
        let medium = if current.medium_busy {
            format!("busy ({})", current.medium_busy_slots_remaining)
        } else {
            format!("idle ({})", current.idle_slots)
        };
        let lines = vec![
            Line::from(Span::styled(
                pane.label.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(format!("event: {}", event_label(current))),
            Line::from(format!("medium: {}", medium)),
            Line::from(format!(
                "success={} collisions={}",
                aggregate.total_successful_packets, aggregate.collision_events
            )),
            Line::from(format!(
                "delay={:.2} throughput={:.2}",
                aggregate.average_delay_slots, aggregate.throughput_bits_per_slot
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

    fn render_sparklines(&self, frame: &mut Frame, area: Rect, pane_index: usize) {
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Min(1),
            ])
            .split(area);

        let throughput = self.sparkline_data(pane_index, SparklineMetric::Throughput);
        let collisions = self.sparkline_data(pane_index, SparklineMetric::Collisions);
        let successes = self.sparkline_data(pane_index, SparklineMetric::Successes);

        frame.render_widget(
            Sparkline::default()
                .block(Block::default().borders(Borders::ALL).title("Throughput"))
                .data(throughput.as_slice())
                .style(Style::default().fg(Color::Cyan))
                .max(*throughput.iter().max().unwrap_or(&1)),
            sections[0],
        );
        frame.render_widget(
            Sparkline::default()
                .block(Block::default().borders(Borders::ALL).title("Collisions"))
                .data(collisions.as_slice())
                .style(Style::default().fg(Color::Red))
                .max(*collisions.iter().max().unwrap_or(&1)),
            sections[1],
        );
        frame.render_widget(
            Sparkline::default()
                .block(Block::default().borders(Borders::ALL).title("Successes"))
                .data(successes.as_slice())
                .style(Style::default().fg(Color::Green))
                .max(*successes.iter().max().unwrap_or(&1)),
            sections[2],
        );
    }

    fn sparkline_data(&self, pane_index: usize, metric: SparklineMetric) -> Vec<u64> {
        let pane = &self.panes[pane_index];
        let upto = self
            .frame_index
            .min(pane.trace.frames.len().saturating_sub(1));
        pane.trace.frames[..=upto]
            .iter()
            .map(|frame| match metric {
                SparklineMetric::Throughput => {
                    frame.progress.aggregate.throughput_bits_per_slot.round() as u64
                }
                SparklineMetric::Collisions => frame.progress.aggregate.collision_events,
                SparklineMetric::Successes => frame.progress.aggregate.total_successful_packets,
            })
            .collect()
    }

    fn render_class_table(&self, frame: &mut Frame, area: Rect, pane_index: usize) {
        let rows = self
            .current_frame(pane_index)
            .progress
            .per_class
            .iter()
            .map(|class| {
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
                Constraint::Length(10),
                Constraint::Length(5),
                Constraint::Length(6),
                Constraint::Length(7),
                Constraint::Length(8),
                Constraint::Length(9),
            ],
        )
        .header(
            Row::new(vec!["class", "users", "sent", "coll", "delay", "thrpt"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(Block::default().borders(Borders::ALL).title("Per Class"));

        frame.render_widget(table, area);
    }

    fn render_station_table(&self, frame: &mut Frame, area: Rect, pane_index: usize) {
        let rows = self
            .current_frame(pane_index)
            .stations
            .iter()
            .map(|station| {
                Row::new(vec![
                    Cell::from(station.id.to_string()),
                    Cell::from(station.class_name.clone()),
                    Cell::from(phase_label(station.phase)),
                    Cell::from(station.backoff_counter.to_string()),
                    Cell::from(station.current_cw.to_string()),
                    Cell::from(station.defer_slots_remaining.to_string()),
                    Cell::from(station.successful_packets.to_string()),
                    Cell::from(station.collision_attempts.to_string()),
                ])
                .style(phase_style(station.phase))
            });

        let table = Table::new(
            rows,
            [
                Constraint::Length(3),
                Constraint::Length(10),
                Constraint::Length(9),
                Constraint::Length(5),
                Constraint::Length(4),
                Constraint::Length(5),
                Constraint::Length(4),
                Constraint::Length(5),
            ],
        )
        .header(
            Row::new(vec!["id", "class", "phase", "bo", "cw", "dfr", "tx", "col"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(Block::default().borders(Borders::ALL).title("Stations"));

        frame.render_widget(table, area);
    }

    fn render_teaching(&self, frame: &mut Frame, area: Rect) {
        if !self.teaching_mode {
            frame.render_widget(
                Paragraph::new("teaching mode is off. press t to show captions.")
                    .block(Block::default().borders(Borders::ALL).title("Teaching")),
                area,
            );
            return;
        }

        let areas = if self.panes.len() > 1 {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(100)])
                .split(area)
        };

        for (index, pane_area) in areas.iter().enumerate() {
            let pane = &self.panes[index];
            let text = vec![
                Line::from(Span::styled(
                    pane.label.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(teaching_caption(self.current_frame(index))),
            ];
            frame.render_widget(
                Paragraph::new(text)
                    .block(Block::default().borders(Borders::ALL).title("Teaching"))
                    .wrap(Wrap { trim: true }),
                *pane_area,
            );
        }
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let sections = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(68), Constraint::Percentage(32)])
            .split(area);

        self.render_event_log(frame, sections[0]);
        self.render_controls(frame, sections[1]);
    }

    fn render_controls(&self, frame: &mut Frame, area: Rect) {
        let lines = vec![
            Line::from("space: pause/resume"),
            Line::from("n: step one slot"),
            Line::from("f: faster playback"),
            Line::from("s: slower playback"),
            Line::from("r: restart"),
            Line::from("q: quit"),
            Line::from("t: toggle teaching"),
        ];

        frame.render_widget(
            Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title("Controls"))
                .wrap(Wrap { trim: true }),
            area,
        );
    }

    fn render_event_log(&self, frame: &mut Frame, area: Rect) {
        let mut items = Vec::new();

        for (pane_index, pane) in self.panes.iter().enumerate() {
            items.push(ListItem::new(Line::from(Span::styled(
                pane.label.clone(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ))));
            items.extend(
                self.recent_events(pane_index)
                    .into_iter()
                    .map(ListItem::new),
            );
        }

        frame.render_widget(
            List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Recent Events"),
            ),
            area,
        );
    }

    fn recent_events(&self, pane_index: usize) -> Vec<String> {
        let pane = &self.panes[pane_index];
        let upto = self
            .frame_index
            .min(pane.trace.frames.len().saturating_sub(1));
        let start = upto.saturating_sub(4);
        pane.trace.frames[start..=upto]
            .iter()
            .map(|trace_frame| {
                format!("slot {:>3}: {}", trace_frame.slot, event_label(trace_frame))
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy)]
enum SparklineMetric {
    Throughput,
    Collisions,
    Successes,
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

fn teaching_caption(frame: &TraceFrame) -> String {
    match &frame.event {
        crate::sim::dcf::engine::SlotEvent::Busy {
            busy_slots_remaining,
        } => format!(
            "The medium is still occupied, so contenders freeze their backoff counters and wait. After {} more busy slots, stations will satisfy the defer interval before counting down again.",
            busy_slots_remaining
        ),
        crate::sim::dcf::engine::SlotEvent::Idle => {
            let deferring = frame
                .stations
                .iter()
                .filter(|station| station.phase == crate::sim::dcf::phase::StationPhase::Defer)
                .count();
            let counting_down = frame
                .stations
                .iter()
                .filter(|station| {
                    station.phase == crate::sim::dcf::phase::StationPhase::BackoffCountdown
                })
                .count();

            if deferring > 0 {
                format!(
                    "The channel is idle, but {} station(s) are still satisfying the DIFS-style defer period before they can decrement backoff.",
                    deferring
                )
            } else {
                format!(
                    "The channel is idle and {} station(s) are counting down backoff. A station transmits when its counter reaches zero in an eligible slot.",
                    counting_down
                )
            }
        }
        crate::sim::dcf::engine::SlotEvent::Success { station_id } => format!(
            "Only station {} reached zero this slot, so the attempt succeeded. Its contention window resets to CWmin and the next frame starts a fresh defer-and-backoff cycle.",
            station_id
        ),
        crate::sim::dcf::engine::SlotEvent::Collision { station_ids } => format!(
            "Stations {:?} transmitted together, so they infer a collision. They increase CW using binary exponential backoff, redraw counters, and retry after the medium becomes idle again.",
            station_ids
        ),
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

fn cw_label(scenario: &Scenario) -> String {
    scenario
        .classes
        .iter()
        .map(|class| format!("{}:{}", class.name, class.cw_min))
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use crate::sim::trace;

    use super::{DemoPreset, build_compare_scenario, build_demo_scenario, read_trace, write_trace};

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

    #[test]
    fn compare_cw_min_scales_mixed_preset() {
        let base = build_demo_scenario(DemoPreset::Mixed, 7, 120);
        let compare = build_compare_scenario(&base, None, Some(8)).expect("compare should build");

        assert_eq!(compare.classes[0].cw_min, 8);
        assert_eq!(compare.classes[1].cw_min, 32);
    }

    #[test]
    fn trace_export_round_trip_preserves_frame_count() {
        let scenario = build_demo_scenario(DemoPreset::Collision, 7, 24);
        let trace = trace(&scenario).expect("trace should build");
        let path = std::env::temp_dir().join(format!(
            "csma_ca_trace_{}_{}.json",
            std::process::id(),
            trace.frames.len()
        ));

        write_trace(&path, &trace).expect("trace should write");
        let loaded = read_trace(&path).expect("trace should read");
        let _ = std::fs::remove_file(&path);

        assert_eq!(loaded.frames.len(), trace.frames.len());
        assert_eq!(loaded.report, trace.report);
    }
}
