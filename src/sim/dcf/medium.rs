#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediumState {
    pub busy_slots_remaining: u32,
    pub idle_slots: u32,
}

impl MediumState {
    pub fn idle() -> Self {
        Self {
            busy_slots_remaining: 0,
            idle_slots: 0,
        }
    }

    pub fn is_busy(&self) -> bool {
        self.busy_slots_remaining > 0
    }

    pub fn start_busy(&mut self, busy_slots_remaining: u32) {
        self.busy_slots_remaining = busy_slots_remaining;
        self.idle_slots = 0;
    }

    pub fn consume_busy_slot(&mut self) {
        if self.busy_slots_remaining > 0 {
            self.busy_slots_remaining -= 1;
        }
        self.idle_slots = 0;
    }

    pub fn observe_idle_slot(&mut self) {
        self.idle_slots = self.idle_slots.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::MediumState;

    #[test]
    fn busy_slots_count_down_to_idle() {
        let mut medium = MediumState::idle();

        medium.start_busy(2);
        assert!(medium.is_busy());

        medium.consume_busy_slot();
        assert!(medium.is_busy());

        medium.consume_busy_slot();
        assert!(!medium.is_busy());
    }

    #[test]
    fn idle_slots_reset_when_busy_starts() {
        let mut medium = MediumState::idle();
        medium.observe_idle_slot();
        medium.observe_idle_slot();

        medium.start_busy(1);

        assert_eq!(medium.idle_slots, 0);
    }
}
