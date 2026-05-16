#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentionWindow {
    pub cw_min: u32,
    pub current: u32,
    pub cw_max: u32,
}

impl ContentionWindow {
    pub fn new(cw_min: u32, cw_max: u32) -> Self {
        Self {
            cw_min,
            current: cw_min,
            cw_max,
        }
    }

    pub fn reset(&mut self) {
        self.current = self.cw_min;
    }

    pub fn increase_binary_exponential(&mut self) {
        self.current = self
            .current
            .saturating_mul(2)
            .saturating_add(1)
            .min(self.cw_max);
    }
}

#[cfg(test)]
mod tests {
    use super::ContentionWindow;

    #[test]
    fn increase_binary_exponential_uses_spec_rule() {
        let mut window = ContentionWindow::new(15, 1023);

        window.increase_binary_exponential();

        assert_eq!(window.current, 31);
    }

    #[test]
    fn increase_binary_exponential_caps_at_maximum() {
        let mut window = ContentionWindow::new(31, 63);
        window.current = 63;

        window.increase_binary_exponential();

        assert_eq!(window.current, 63);
    }
}
