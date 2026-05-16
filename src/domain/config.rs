use serde::{Deserialize, Serialize};

use crate::domain::scenario::{Scenario, StationClass, TimingConfig, WindowConfig};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StationClassConfig {
    pub name: String,
    pub users: u32,
    pub cw_min: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SimulationConfig {
    pub total_slots: u64,
    pub payload_bits: u64,
    pub cw_max: u32,
    pub seed: u64,
    pub classes: Vec<StationClassConfig>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct SimulationSettings {
    pub total_slots: u64,
    pub payload_bits: u64,
    pub cw_max: u32,
    pub seed: u64,
}

impl SimulationConfig {
    pub fn standard(users: u32, cw_min: u32, settings: SimulationSettings) -> Self {
        Self {
            total_slots: settings.total_slots,
            payload_bits: settings.payload_bits,
            cw_max: settings.cw_max,
            seed: settings.seed,
            classes: vec![StationClassConfig {
                name: String::from("standard"),
                users,
                cw_min,
            }],
        }
    }

    pub fn mixed(
        lower_users: u32,
        higher_users: u32,
        lower_cw_min: u32,
        higher_cw_min: u32,
        settings: SimulationSettings,
    ) -> Self {
        Self {
            total_slots: settings.total_slots,
            payload_bits: settings.payload_bits,
            cw_max: settings.cw_max,
            seed: settings.seed,
            classes: vec![
                StationClassConfig {
                    name: String::from("lower-cw"),
                    users: lower_users,
                    cw_min: lower_cw_min,
                },
                StationClassConfig {
                    name: String::from("higher-cw"),
                    users: higher_users,
                    cw_min: higher_cw_min,
                },
            ],
        }
    }

    pub fn total_users(&self) -> u32 {
        self.classes.iter().map(|class| class.users).sum()
    }

    pub fn to_scenario(&self) -> Scenario {
        Scenario {
            seed: self.seed,
            timing: TimingConfig {
                total_slots: self.total_slots,
                payload_bits: self.payload_bits,
                difs_slots: 1,
                sifs_slots: 0,
                tx_duration_slots: 1,
            },
            window: WindowConfig {
                cw_max: self.cw_max,
            },
            classes: self
                .classes
                .iter()
                .map(|class| StationClass {
                    name: class.name.clone(),
                    users: class.users,
                    cw_min: class.cw_min,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SweepParameters {
    pub total_slots: u64,
    pub payload_bits: u64,
    pub cw_max: u32,
    pub trials: u32,
    pub base_seed: u64,
}

impl Default for SweepParameters {
    fn default() -> Self {
        Self {
            total_slots: 20_000,
            payload_bits: 12_000,
            cw_max: 1_024,
            trials: 5,
            base_seed: 7,
        }
    }
}
