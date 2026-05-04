#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StationState {
    pub id: usize,
    pub class_name: String,
    pub cw_min: u32,
    pub current_cw: u32,
    pub backoff: u32,
    pub packet_age_slots: u64,
    pub successful_packets: u64,
    pub collision_attempts: u64,
    pub total_delay_slots: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransmissionOutcome {
    Idle,
    Success { station_id: usize },
    Collision { station_ids: Vec<usize> },
}
