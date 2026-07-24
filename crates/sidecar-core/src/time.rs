use serde::{Deserialize, Serialize};

/// A wall-clock Unix timestamp in milliseconds.
///
/// It is intentionally represented without a timezone library so protocol
/// contracts remain lightweight. Display layers are responsible for formatting.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct UnixTimestampMillis(pub i64);

/// A caller-supplied monotonic timestamp in milliseconds.
///
/// This must never be persisted as wall-clock time. It exists for deterministic
/// input recognition, deadlines, and latency measurement.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct MonotonicMillis(pub u64);

impl MonotonicMillis {
    pub fn saturating_duration_since(self, earlier: Self) -> u64 {
        self.0.saturating_sub(earlier.0)
    }

    #[must_use]
    pub fn saturating_add(self, duration_ms: u64) -> Self {
        Self(self.0.saturating_add(duration_ms))
    }
}
