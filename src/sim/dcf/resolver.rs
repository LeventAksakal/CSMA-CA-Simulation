#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransmissionResolution {
    Idle,
    Success { station_id: usize },
    Collision { station_ids: Vec<usize> },
}

pub fn resolve_transmission(contenders: Vec<usize>) -> TransmissionResolution {
    match contenders.as_slice() {
        [] => TransmissionResolution::Idle,
        [station_id] => TransmissionResolution::Success {
            station_id: *station_id,
        },
        _ => TransmissionResolution::Collision {
            station_ids: contenders,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{TransmissionResolution, resolve_transmission};

    #[test]
    fn no_contenders_means_idle() {
        assert_eq!(
            resolve_transmission(Vec::new()),
            TransmissionResolution::Idle
        );
    }

    #[test]
    fn one_contender_means_success() {
        assert_eq!(
            resolve_transmission(vec![4]),
            TransmissionResolution::Success { station_id: 4 }
        );
    }

    #[test]
    fn multiple_contenders_mean_collision() {
        assert_eq!(
            resolve_transmission(vec![1, 2]),
            TransmissionResolution::Collision {
                station_ids: vec![1, 2]
            }
        );
    }
}
