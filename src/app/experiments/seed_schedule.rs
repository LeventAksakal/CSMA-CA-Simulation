use anyhow::{Result, ensure};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrialSeedSchedule {
    pub base_seed: u64,
    pub trials: u32,
}

impl TrialSeedSchedule {
    pub fn new(base_seed: u64, trials: u32) -> Result<Self> {
        ensure!(trials > 0, "trials must be greater than zero");
        Ok(Self { base_seed, trials })
    }

    pub fn seed_for_trial(&self, trial: u32) -> Result<u64> {
        ensure!(trial < self.trials, "trial index out of range");
        self.base_seed
            .checked_add(u64::from(trial))
            .ok_or_else(|| anyhow::anyhow!("seed overflow while deriving trial seed"))
    }
}
