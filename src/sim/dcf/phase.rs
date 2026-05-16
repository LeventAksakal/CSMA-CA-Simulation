use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StationPhase {
    WaitingForMedium,
    Defer,
    BackoffCountdown,
    Transmitting,
    AwaitingResult,
    CollisionRecovery,
}
