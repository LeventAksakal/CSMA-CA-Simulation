use super::{backoff::BackoffState, phase::StationPhase, window::ContentionWindow};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StationRuntime {
    pub id: usize,
    pub class_name: String,
    pub phase: StationPhase,
    pub cw_min: u32,
    pub window: ContentionWindow,
    pub backoff: BackoffState,
    pub defer_slots_remaining: u32,
    pub packet_age_slots: u64,
    pub successful_packets: u64,
    pub collision_attempts: u64,
    pub total_delay_slots: u64,
}

impl StationRuntime {
    pub fn wait_for_medium(&mut self) {
        self.defer_slots_remaining = 0;
        self.phase = StationPhase::WaitingForMedium;
    }

    pub fn enter_defer(&mut self, defer_slots: u32) {
        self.defer_slots_remaining = defer_slots;
        self.phase = if defer_slots == 0 {
            self.backoff.resume();
            StationPhase::BackoffCountdown
        } else {
            StationPhase::Defer
        };
    }

    pub fn freeze_for_busy(&mut self) {
        if self.phase == StationPhase::BackoffCountdown {
            self.backoff.freeze();
        }

        self.wait_for_medium();
    }

    pub fn advance_idle_slot(&mut self) {
        match self.phase {
            StationPhase::Defer => {
                if self.defer_slots_remaining > 0 {
                    self.defer_slots_remaining -= 1;
                }

                if self.defer_slots_remaining == 0 {
                    self.backoff.resume();
                    self.phase = StationPhase::BackoffCountdown;
                }
            }
            StationPhase::BackoffCountdown => self.backoff.decrement(),
            _ => {}
        }
    }

    pub fn is_ready_to_transmit(&self) -> bool {
        self.phase == StationPhase::BackoffCountdown && self.backoff_counter() == 0
    }

    pub fn current_cw(&self) -> u32 {
        self.window.current
    }

    pub fn backoff_counter(&self) -> u32 {
        self.backoff.counter
    }
}

#[cfg(test)]
mod tests {
    use super::StationRuntime;
    use crate::sim::dcf::{backoff::BackoffState, phase::StationPhase, window::ContentionWindow};

    fn station() -> StationRuntime {
        StationRuntime {
            id: 0,
            class_name: String::from("standard"),
            phase: StationPhase::BackoffCountdown,
            cw_min: 15,
            window: ContentionWindow::new(15, 1023),
            backoff: BackoffState::new(4),
            defer_slots_remaining: 0,
            packet_age_slots: 0,
            successful_packets: 0,
            collision_attempts: 0,
            total_delay_slots: 0,
        }
    }

    #[test]
    fn enter_defer_uses_defer_phase_when_slots_remain() {
        let mut station = station();

        station.enter_defer(2);

        assert_eq!(station.phase, StationPhase::Defer);
        assert_eq!(station.defer_slots_remaining, 2);
    }

    #[test]
    fn freeze_for_busy_preserves_backoff_counter() {
        let mut station = station();

        station.freeze_for_busy();
        station.backoff.decrement();
        station.enter_defer(1);
        station.advance_idle_slot();

        assert_eq!(station.phase, StationPhase::BackoffCountdown);
        assert_eq!(station.backoff.counter, 4);
    }

    #[test]
    fn idle_slot_counts_down_defer_before_backoff() {
        let mut station = station();
        station.enter_defer(1);

        station.advance_idle_slot();
        assert_eq!(station.backoff.counter, 4);

        station.advance_idle_slot();
        assert_eq!(station.backoff.counter, 3);
    }
}
