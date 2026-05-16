use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StationClass {
    pub name: String,
    pub users: u32,
    pub cw_min: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimingConfig {
    pub total_slots: u64,
    pub payload_bits: u64,
    pub difs_slots: u32,
    pub sifs_slots: u32,
    pub tx_duration_slots: u32,
    pub collision_penalty_slots: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowConfig {
    pub cw_max: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Scenario {
    pub seed: u64,
    pub timing: TimingConfig,
    pub window: WindowConfig,
    pub classes: Vec<StationClass>,
}

impl Scenario {
    pub fn standard(users: u32, cw_min: u32, seed: u64, timing: TimingConfig, cw_max: u32) -> Self {
        Self {
            seed,
            timing,
            window: WindowConfig { cw_max },
            classes: vec![StationClass {
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
        seed: u64,
        timing: TimingConfig,
        cw_max: u32,
    ) -> Self {
        Self {
            seed,
            timing,
            window: WindowConfig { cw_max },
            classes: vec![
                StationClass {
                    name: String::from("lower-cw"),
                    users: lower_users,
                    cw_min: lower_cw_min,
                },
                StationClass {
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
}
