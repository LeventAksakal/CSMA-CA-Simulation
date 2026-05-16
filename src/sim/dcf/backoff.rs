#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackoffState {
    pub counter: u32,
    pub frozen_counter: Option<u32>,
}

impl BackoffState {
    pub fn new(counter: u32) -> Self {
        Self {
            counter,
            frozen_counter: None,
        }
    }

    pub fn decrement(&mut self) {
        if self.counter > 0 {
            self.counter -= 1;
        }
    }

    pub fn freeze(&mut self) {
        self.frozen_counter = Some(self.counter);
    }

    pub fn resume(&mut self) {
        if let Some(counter) = self.frozen_counter.take() {
            self.counter = counter;
        }
    }

    pub fn replace(&mut self, counter: u32) {
        self.counter = counter;
        self.frozen_counter = None;
    }
}

#[cfg(test)]
mod tests {
    use super::BackoffState;

    #[test]
    fn decrement_stops_at_zero() {
        let mut backoff = BackoffState::new(1);

        backoff.decrement();
        backoff.decrement();

        assert_eq!(backoff.counter, 0);
    }

    #[test]
    fn freeze_and_resume_restore_counter() {
        let mut backoff = BackoffState::new(7);

        backoff.freeze();
        backoff.decrement();
        backoff.resume();

        assert_eq!(backoff.counter, 7);
        assert_eq!(backoff.frozen_counter, None);
    }

    #[test]
    fn replace_clears_frozen_counter() {
        let mut backoff = BackoffState::new(3);
        backoff.freeze();

        backoff.replace(9);

        assert_eq!(backoff.counter, 9);
        assert_eq!(backoff.frozen_counter, None);
    }
}
