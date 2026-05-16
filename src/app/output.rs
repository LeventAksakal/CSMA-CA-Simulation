use std::{fs, path::Path};

use anyhow::Result;
use csv::Writer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExperimentRecord {
    pub scenario: String,
    pub trial: u32,
    pub seed: u64,
    pub total_users: u32,
    pub cw_min: Option<u32>,
    pub lower_cw_min: Option<u32>,
    pub higher_cw_min: Option<u32>,
    pub class_name: String,
    pub class_users: u32,
    pub successful_packets: u64,
    pub collision_attempts: u64,
    pub average_delay_slots: f64,
    pub throughput_bits_per_slot: f64,
}

pub fn write_csv(path: &Path, records: &[ExperimentRecord]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut writer = Writer::from_path(path)?;

    for record in records {
        writer.serialize(record)?;
    }

    writer.flush()?;
    Ok(())
}
