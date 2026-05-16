use std::path::PathBuf;

use anyhow::{Result, ensure};
use clap::{Parser, Subcommand};

use crate::{
    app::{
        experiments,
        output::write_csv,
        plot, report,
        summary::{self, summarize_records},
        tui,
    },
    domain::config::{SimulationConfig, SimulationSettings, SweepParameters, TimingPreset},
    sim::simulate,
};

#[derive(Debug, Parser)]
#[command(author, version, about = "CSMA/CA simulator and experiment runner")]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Single {
        #[arg(long)]
        users: u32,
        #[arg(long)]
        cw_min: u32,
        #[arg(long, default_value_t = 1024)]
        cw_max: u32,
        #[arg(long, default_value_t = 20_000)]
        slots: u64,
        #[arg(long, default_value_t = 12_000)]
        payload_bits: u64,
        #[arg(long, default_value_t = 7)]
        seed: u64,
        #[arg(long, value_enum, default_value_t = TimingPreset::Baseline)]
        timing_preset: TimingPreset,
    },
    SweepUsers {
        #[arg(long)]
        min_users: u32,
        #[arg(long)]
        max_users: u32,
        #[arg(long, default_value_t = 5)]
        step: u32,
        #[arg(long)]
        cw_min: u32,
        #[arg(long, default_value_t = 1024)]
        cw_max: u32,
        #[arg(long, default_value_t = 20_000)]
        slots: u64,
        #[arg(long, default_value_t = 12_000)]
        payload_bits: u64,
        #[arg(long, default_value_t = 5)]
        trials: u32,
        #[arg(long, default_value_t = 7)]
        seed: u64,
        #[arg(long, value_enum, default_value_t = TimingPreset::Baseline)]
        timing_preset: TimingPreset,
        #[arg(long)]
        output: PathBuf,
    },
    SweepCw {
        #[arg(long)]
        users: u32,
        #[arg(long)]
        min_cw: Option<u32>,
        #[arg(long)]
        max_cw: Option<u32>,
        #[arg(long, default_value_t = 4)]
        step: u32,
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        cw_values: Option<Vec<u32>>,
        #[arg(long, default_value_t = 1024)]
        cw_max: u32,
        #[arg(long, default_value_t = 20_000)]
        slots: u64,
        #[arg(long, default_value_t = 12_000)]
        payload_bits: u64,
        #[arg(long, default_value_t = 5)]
        trials: u32,
        #[arg(long, default_value_t = 7)]
        seed: u64,
        #[arg(long, value_enum, default_value_t = TimingPreset::Baseline)]
        timing_preset: TimingPreset,
        #[arg(long)]
        output: PathBuf,
    },
    MixedClasses {
        #[arg(long)]
        lower_users: u32,
        #[arg(long)]
        higher_users: u32,
        #[arg(long)]
        lower_cw_min: u32,
        #[arg(long)]
        higher_cw_min: u32,
        #[arg(long, default_value_t = 1024)]
        cw_max: u32,
        #[arg(long, default_value_t = 20_000)]
        slots: u64,
        #[arg(long, default_value_t = 12_000)]
        payload_bits: u64,
        #[arg(long, default_value_t = 5)]
        trials: u32,
        #[arg(long, default_value_t = 7)]
        seed: u64,
        #[arg(long, value_enum, default_value_t = TimingPreset::Baseline)]
        timing_preset: TimingPreset,
        #[arg(long)]
        output: PathBuf,
    },
    Plot {
        #[arg(long)]
        users_input: PathBuf,
        #[arg(long)]
        cw_input: PathBuf,
        #[arg(long)]
        mixed_input: PathBuf,
        #[arg(long)]
        output_dir: PathBuf,
    },
    Report {
        #[arg(long)]
        users_input: PathBuf,
        #[arg(long)]
        cw_input: PathBuf,
        #[arg(long)]
        mixed_input: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        plots_dir: Option<PathBuf>,
    },
    Demo {
        #[arg(long, default_value = "mixed")]
        preset: tui::DemoPreset,
        #[arg(long, default_value_t = 180)]
        slots: u64,
        #[arg(long, default_value_t = 7)]
        seed: u64,
        #[arg(long, default_value_t = 150)]
        tick_ms: u64,
        #[arg(long)]
        replay: Option<PathBuf>,
        #[arg(long)]
        export_trace: Option<PathBuf>,
        #[arg(long)]
        compare_seed: Option<u64>,
        #[arg(long)]
        compare_cw_min: Option<u32>,
        #[arg(long)]
        compare_replay: Option<PathBuf>,
    },
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Single {
            users,
            cw_min,
            cw_max,
            slots,
            payload_bits,
            seed,
            timing_preset,
        } => {
            let config = SimulationConfig::standard(
                users,
                cw_min,
                SimulationSettings {
                    total_slots: slots,
                    payload_bits,
                    cw_max,
                    seed,
                    timing_preset,
                },
            );
            let result = simulate(&config)?;

            println!(
                "aggregate.successful_packets={}",
                result.aggregate.total_successful_packets
            );
            println!(
                "aggregate.collision_events={}",
                result.aggregate.collision_events
            );
            println!(
                "aggregate.average_delay_slots={:.4}",
                result.aggregate.average_delay_slots
            );
            println!(
                "aggregate.throughput_bits_per_slot={:.4}",
                result.aggregate.throughput_bits_per_slot
            );

            for class in result.per_class {
                println!(
                    "class={} users={} success={} collisions={} avg_delay={:.4} throughput={:.4}",
                    class.class_name,
                    class.users,
                    class.successful_packets,
                    class.collision_attempts,
                    class.average_delay_slots,
                    class.throughput_bits_per_slot
                );
            }
        }
        Command::SweepUsers {
            min_users,
            max_users,
            step,
            cw_min,
            cw_max,
            slots,
            payload_bits,
            trials,
            seed,
            timing_preset,
            output,
        } => {
            let params =
                build_sweep_parameters(cw_max, slots, payload_bits, trials, seed, timing_preset)?;
            let records = experiments::sweep_users(min_users, max_users, step, cw_min, &params)?;
            let summary_records = summarize_records(&records);
            let summary_output = summary::summary_output_path(&output);
            write_csv(&output, &records)?;
            write_csv(&summary_output, &summary_records)?;
            println!("wrote {} records to {}", records.len(), output.display());
            println!(
                "wrote {} summary rows to {}",
                summary_records.len(),
                summary_output.display()
            );
        }
        Command::SweepCw {
            users,
            min_cw,
            max_cw,
            step,
            cw_values,
            cw_max,
            slots,
            payload_bits,
            trials,
            seed,
            timing_preset,
            output,
        } => {
            let params =
                build_sweep_parameters(cw_max, slots, payload_bits, trials, seed, timing_preset)?;
            let cw_values = resolve_cw_sweep_values(min_cw, max_cw, step, cw_values)?;
            let records = experiments::sweep_cw_values(users, &cw_values, &params)?;
            let summary_records = summarize_records(&records);
            let summary_output = summary::summary_output_path(&output);
            write_csv(&output, &records)?;
            write_csv(&summary_output, &summary_records)?;
            println!("wrote {} records to {}", records.len(), output.display());
            println!(
                "wrote {} summary rows to {}",
                summary_records.len(),
                summary_output.display()
            );
        }
        Command::MixedClasses {
            lower_users,
            higher_users,
            lower_cw_min,
            higher_cw_min,
            cw_max,
            slots,
            payload_bits,
            trials,
            seed,
            timing_preset,
            output,
        } => {
            let params =
                build_sweep_parameters(cw_max, slots, payload_bits, trials, seed, timing_preset)?;
            let records = experiments::mixed_classes(
                lower_users,
                higher_users,
                lower_cw_min,
                higher_cw_min,
                &params,
            )?;
            let summary_records = summarize_records(&records);
            let summary_output = summary::summary_output_path(&output);
            write_csv(&output, &records)?;
            write_csv(&summary_output, &summary_records)?;
            println!("wrote {} records to {}", records.len(), output.display());
            println!(
                "wrote {} summary rows to {}",
                summary_records.len(),
                summary_output.display()
            );
        }
        Command::Plot {
            users_input,
            cw_input,
            mixed_input,
            output_dir,
        } => {
            let outputs = plot::write_plots(&users_input, &cw_input, &mixed_input, &output_dir)?;

            for output in outputs {
                println!("wrote {}", output.display());
            }
        }
        Command::Report {
            users_input,
            cw_input,
            mixed_input,
            output,
            plots_dir,
        } => {
            report::write_report(
                &users_input,
                &cw_input,
                &mixed_input,
                &output,
                plots_dir.as_deref(),
            )?;
            println!("wrote {}", output.display());
        }
        Command::Demo {
            preset,
            slots,
            seed,
            tick_ms,
            replay,
            export_trace,
            compare_seed,
            compare_cw_min,
            compare_replay,
        } => tui::run_demo(tui::DemoOptions {
            preset,
            seed,
            slots,
            tick_ms,
            replay,
            export_trace,
            compare_seed,
            compare_cw_min,
            compare_replay,
        })?,
    }

    Ok(())
}

fn resolve_cw_sweep_values(
    min_cw: Option<u32>,
    max_cw: Option<u32>,
    step: u32,
    cw_values: Option<Vec<u32>>,
) -> Result<Vec<u32>> {
    if let Some(cw_values) = cw_values {
        ensure!(!cw_values.is_empty(), "cw-values must not be empty");
        return Ok(cw_values);
    }

    if min_cw.is_none() && max_cw.is_none() {
        return Ok(vec![0, 2, 4, 8, 16, 32, 64]);
    }

    let min_cw = min_cw.unwrap_or(0);
    let max_cw = max_cw.unwrap_or(64);
    ensure!(step > 0, "step must be greater than zero");
    ensure!(
        min_cw <= max_cw,
        "min-cw must be less than or equal to max-cw"
    );

    let mut values = Vec::new();
    let mut current = min_cw;

    while current <= max_cw {
        values.push(current);

        match current.checked_add(step) {
            Some(next) if next > current => current = next,
            _ => break,
        }
    }

    Ok(values)
}

fn build_sweep_parameters(
    cw_max: u32,
    total_slots: u64,
    payload_bits: u64,
    trials: u32,
    base_seed: u64,
    timing_preset: TimingPreset,
) -> Result<SweepParameters> {
    ensure!(trials > 0, "trials must be greater than zero");

    Ok(SweepParameters {
        total_slots,
        payload_bits,
        cw_max,
        trials,
        base_seed,
        timing_preset,
    })
}

#[cfg(test)]
mod tests {
    use super::resolve_cw_sweep_values;

    #[test]
    fn cw_sweep_defaults_to_seeded_values_range() {
        let values = resolve_cw_sweep_values(None, None, 8, None)
            .expect("default cw sweep values should expand");

        assert_eq!(values, vec![0, 2, 4, 8, 16, 32, 64]);
    }

    #[test]
    fn cw_sweep_accepts_explicit_seeded_values() {
        let values = resolve_cw_sweep_values(None, None, 8, Some(vec![0, 2, 4, 8, 16, 32, 64]))
            .expect("explicit cw values should be accepted");

        assert_eq!(values, vec![0, 2, 4, 8, 16, 32, 64]);
    }
}
