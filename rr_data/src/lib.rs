/// A date-time represented as nanoseconds since unix epoch
#[derive(Copy, Clone, Debug, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Time(i64);

impl Time {
    pub fn now() -> Self {
        Self(nanos_since_epoch())
    }
}

/// Returns a high-precision, monotonically increasing nanosecond count since unix epoch.
#[inline]
pub fn nanos_since_epoch() -> i64 {
    // This can maybe be optimized
    use once_cell::sync::Lazy;
    use std::time::Instant;

    fn epoch_offset_and_start() -> (i64, Instant) {
        if let Ok(duration_since_epoch) = std::time::UNIX_EPOCH.elapsed() {
            let nanos_since_epoch = duration_since_epoch.as_nanos() as i64;
            (nanos_since_epoch, Instant::now())
        } else {
            // system time is set before 1970. this should be quite rare.
            (0, Instant::now())
        }
    }

    static START_TIME: Lazy<(i64, Instant)> = Lazy::new(epoch_offset_and_start);
    START_TIME.0 + START_TIME.1.elapsed().as_nanos() as i64
}

// ----------------------------------------------------------------------------

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Message {
    /// When the data was logged on the client.
    pub log_time: Time,
    pub msg_enum: MessageEnum,
}

impl Message {
    pub fn now(msg_enum: MessageEnum) -> Self {
        Self {
            log_time: Time::now(),
            msg_enum,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        use bincode::Options as _;
        bincode::DefaultOptions::new().serialize(self).unwrap()
    }

    pub fn decode(bytes: &[u8]) -> anyhow::Result<Self> {
        use anyhow::Context as _;
        use bincode::Options as _;
        bincode::DefaultOptions::new()
            .deserialize(bytes)
            .context("bincode")
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum MessageEnum {
    // NewCallsite(RrCallsite),
    // NewSpan(RrSpan),
    // EnterSpan(RrSpandId),
    // ExitSpan(RrSpanId),
    DataEvent(DataEvent),
}

/// A place in the source code where we may be logging data from.
// pub struct RrCallsite {
//     pub name: String,
//     pub target: String,
//     pub level: RrLogLevel,
//     pub location: RrLocation,
//     pub fields: RrKvPairs,
//     pub id: RrCallsiteId,
//     pub kind: RrCallsiteKind,
// }

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DataEvent {
    pub callsite: CallsiteId,
    pub fields: Vec<(String, Value)>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Value {
    String(String),
    I64(i64),
    U64(u64),
    F64(f64),
    Bool(bool),
    Debug(String),
    Error {
        description: String,
        details: String,
    },
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CallsiteId(pub u64);
