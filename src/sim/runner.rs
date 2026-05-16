use anyhow::{Result, anyhow, ensure};

use crate::{
    domain::config::SimulationConfig, domain::report::SimulationReport, domain::scenario::Scenario,
};

use super::dcf;

pub use dcf::engine::{SimulationTrace, TraceFrame};

pub fn run(scenario: &Scenario) -> Result<SimulationReport> {
    dcf::engine::run(scenario)
}

pub fn trace(scenario: &Scenario) -> Result<SimulationTrace> {
    dcf::engine::trace(scenario)
}

pub fn simulate(config: &SimulationConfig) -> Result<SimulationReport> {
    run(&config.to_scenario())
}

pub fn validate_range_step(start: u32, end: u32, step: u32) -> Result<Vec<u32>> {
    ensure!(step > 0, "step must be greater than zero");
    ensure!(
        start <= end,
        "range start must be less than or equal to range end"
    );

    let mut values = Vec::new();
    let mut current = start;

    while current <= end {
        values.push(current);

        match current.checked_add(step) {
            Some(next) if next > current => current = next,
            _ => return Err(anyhow!("range overflow while expanding values")),
        }
    }

    Ok(values)
}
