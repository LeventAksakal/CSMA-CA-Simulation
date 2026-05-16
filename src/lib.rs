pub mod app;
pub mod domain;
pub mod sim;

pub use domain::config::{SimulationConfig, SimulationSettings, SweepParameters};
pub use domain::report::{AggregateReport, ClassReport, SimulationReport};
pub use domain::scenario::{Scenario, StationClass, TimingConfig, WindowConfig};
pub use sim::{run, simulate, validate_range_step};
