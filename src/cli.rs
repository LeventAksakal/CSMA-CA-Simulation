use std::path::PathBuf;

use anyhow::{Result, ensure};
use clap::{Parser, Subcommand};

use crate::{
    config::{SimulationConfig, SimulationSettings, SweepParameters},
    experiments::{mixed_classes, sweep_cwmins, sweep_users},
    output::write_csv,
    simulator::simulate,
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
        #[arg(long)]
        output: PathBuf,
    },
    SweepCw {
        #[arg(long)]
        users: u32,
        #[arg(long)]
        min_cw: u32,
        #[arg(long)]
        max_cw: u32,
        #[arg(long, default_value_t = 4)]
        step: u32,
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
        #[arg(long)]
        output: PathBuf,
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
        } => {
            let config = SimulationConfig::standard(
                users,
                cw_min,
                SimulationSettings {
                    total_slots: slots,
                    payload_bits,
                    cw_max,
                    seed,
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
            output,
        } => {
            let params = build_sweep_parameters(cw_max, slots, payload_bits, trials, seed)?;
            let records = sweep_users(min_users, max_users, step, cw_min, &params)?;
            write_csv(&output, &records)?;
            println!("wrote {} records to {}", records.len(), output.display());
        }
        Command::SweepCw {
            users,
            min_cw,
            max_cw,
            step,
            cw_max,
            slots,
            payload_bits,
            trials,
            seed,
            output,
        } => {
            let params = build_sweep_parameters(cw_max, slots, payload_bits, trials, seed)?;
            let records = sweep_cwmins(users, min_cw, max_cw, step, &params)?;
            write_csv(&output, &records)?;
            println!("wrote {} records to {}", records.len(), output.display());
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
            output,
        } => {
            let params = build_sweep_parameters(cw_max, slots, payload_bits, trials, seed)?;
            let records = mixed_classes(
                lower_users,
                higher_users,
                lower_cw_min,
                higher_cw_min,
                &params,
            )?;
            write_csv(&output, &records)?;
            println!("wrote {} records to {}", records.len(), output.display());
        }
    }

    Ok(())
}

fn build_sweep_parameters(
    cw_max: u32,
    total_slots: u64,
    payload_bits: u64,
    trials: u32,
    base_seed: u64,
) -> Result<SweepParameters> {
    ensure!(trials > 0, "trials must be greater than zero");

    Ok(SweepParameters {
        total_slots,
        payload_bits,
        cw_max,
        trials,
        base_seed,
    })
}
