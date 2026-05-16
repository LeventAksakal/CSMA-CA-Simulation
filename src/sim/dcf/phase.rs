#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StationPhase {
    WaitingForMedium,
    Defer,
    BackoffCountdown,
    Transmitting,
    AwaitingResult,
    CollisionRecovery,
}
