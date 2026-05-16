pub mod app;
pub mod domain;
pub mod sim;

pub use domain::config::{SimulationConfig, SimulationSettings, SweepParameters};
pub use domain::report::{AggregateReport, ClassReport, SimulationReport};
pub use domain::scenario::{Scenario, StationClass, TimingConfig, WindowConfig};
pub use sim::{SimulationTrace, TraceFrame, run, simulate, trace, validate_range_step};
