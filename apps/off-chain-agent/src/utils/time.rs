use std::time::UNIX_EPOCH;

/// Local re-definition of the candid SystemTime struct. Candid is no longer a
/// direct dependency of off-chain-agent, so derive serde traits instead of
/// `candid::CandidType`. Callers that previously serialised via candid can
/// still rely on serde for JSON transport.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub struct SystemTime {
    pub nanos_since_epoch: u32,
    pub secs_since_epoch: u64,
}

pub fn system_time_to_custom(time: std::time::SystemTime) -> SystemTime {
    let duration = time
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    SystemTime {
        nanos_since_epoch: duration.subsec_nanos(),
        secs_since_epoch: duration.as_secs(),
    }
}
