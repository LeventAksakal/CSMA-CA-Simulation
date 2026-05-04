use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AggregateMetrics {
    pub total_successful_packets: u64,
    pub collision_events: u64,
    pub average_delay_slots: f64,
    pub throughput_bits_per_slot: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ClassMetrics {
    pub class_name: String,
    pub users: u32,
    pub successful_packets: u64,
    pub collision_attempts: u64,
    pub average_delay_slots: f64,
    pub throughput_bits_per_slot: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SimulationResult {
    pub total_slots: u64,
    pub payload_bits: u64,
    pub aggregate: AggregateMetrics,
    pub per_class: Vec<ClassMetrics>,
}
