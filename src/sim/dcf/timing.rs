use crate::domain::scenario::TimingConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimingModel {
    pub config: TimingConfig,
}

impl TimingModel {
    pub fn new(config: TimingConfig) -> Self {
        Self { config }
    }

    pub fn defer_slots(&self) -> u32 {
        self.config.difs_slots
    }

    pub fn busy_slots_after_success(&self) -> u32 {
        self.config
            .tx_duration_slots
            .saturating_sub(1)
            .saturating_add(self.config.sifs_slots)
    }

    pub fn busy_slots_after_collision(&self) -> u32 {
        self.config.tx_duration_slots.saturating_sub(1)
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::scenario::TimingConfig;

    use super::TimingModel;

    #[test]
    fn success_busy_slots_include_sifs() {
        let timing = TimingModel::new(TimingConfig {
            total_slots: 10,
            payload_bits: 1_500,
            difs_slots: 2,
            sifs_slots: 1,
            tx_duration_slots: 3,
        });

        assert_eq!(timing.busy_slots_after_success(), 3);
    }

    #[test]
    fn collision_busy_slots_ignore_sifs() {
        let timing = TimingModel::new(TimingConfig {
            total_slots: 10,
            payload_bits: 1_500,
            difs_slots: 2,
            sifs_slots: 4,
            tx_duration_slots: 3,
        });

        assert_eq!(timing.busy_slots_after_collision(), 2);
    }
}
