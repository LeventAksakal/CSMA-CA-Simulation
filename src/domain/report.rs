use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AggregateReport {
    pub total_successful_packets: u64,
    pub collision_events: u64,
    pub average_delay_slots: f64,
    pub throughput_bits_per_slot: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassReport {
    pub class_name: String,
    pub users: u32,
    pub successful_packets: u64,
    pub collision_attempts: u64,
    pub average_delay_slots: f64,
    pub throughput_bits_per_slot: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimulationReport {
    pub total_slots: u64,
    pub payload_bits: u64,
    pub aggregate: AggregateReport,
    pub per_class: Vec<ClassReport>,
}
