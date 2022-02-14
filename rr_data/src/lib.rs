use std::sync::Arc;

pub const DEFAULT_PUB_SUB_PORT: u16 = 9002;
pub const DEFAULT_VIEWER_WEB_SERVER_PORT: u16 = 8787;

/// The top-level message sent to/from a pub-sub server
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum PubSubMsg {
    /// A new topic has been created.
    NewTopic(TopicMeta),

    /// A new message of a given topic.
    TopicMsg(TopicId, Arc<[u8]>),

    /// Please tell me about new messages on this topic.
    SubscribeTo(TopicId),

    /// Please tell me about all the topics
    ListTopics,

    /// List of all existing topics
    AllTopics(Vec<TopicMeta>),
}

impl PubSubMsg {
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TopicId(uuid::Uuid);

impl TopicId {
    pub fn random() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TopicMeta {
    pub id: TopicId,
    pub created: Time,
    pub name: String,
}

// ----------------------------------------------------------------------------

/// A date-time represented as nanoseconds since unix epoch
#[derive(
    Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct Time(i64);

impl Time {
    #[inline]
    pub fn now() -> Self {
        Self(nanos_since_epoch())
    }

    #[inline]
    pub fn nanos_since_epoch(&self) -> i64 {
        self.0
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
    NewCallsite(Callsite),

    NewSpan(Span),
    EnterSpan(SpanId),
    ExitSpan(SpanId),
    DestroySpan(SpanId),

    /// A span has been spawned from another.
    SpanFollowsFrom {
        span: SpanId,
        follows: SpanId,
    },

    DataEvent(DataEvent),
}

/// A place in the source code where we may be logging data from.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Callsite {
    pub id: CallsiteId,
    pub kind: CallsiteKind,
    pub name: String,
    pub level: LogLevel,
    pub location: Location,
    /// Names of data that may be provided in later calls
    pub field_names: Vec<String>,
}

/// Describes a source code location.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Location {
    /// e.g. the name of the module/app that produced the log
    pub module: String,
    /// File name
    pub file: Option<String>,
    /// Line number
    pub line: Option<u32>,
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { module, file, line } = self;
        match (file, line) {
            (None, None) => module.fmt(f),
            (Some(file), None) => write!(f, "{} {}", module, file),
            (None, Some(line)) => write!(f, "{}, line {}", module, line),
            (Some(file), Some(line)) => write!(f, "{} {}:{}", module, file, line),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Span {
    pub id: SpanId,
    /// `None` if this is a new root.
    pub parent_span_id: Option<SpanId>,
    pub callsite_id: CallsiteId,
    pub fields: FieldSet,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum LogLevel {
    /// The "trace" level.
    ///
    /// Designates very low priority, often extremely verbose, information.
    Trace = 0,

    /// The "debug" level.
    ///
    /// Designates lower priority information.
    Debug = 1,

    /// The "info" level.
    ///
    /// Designates useful information.
    Info = 2,

    /// The "warn" level.
    ///
    /// Designates hazardous situations.
    Warn = 3,

    /// The "error" level.
    ///
    /// Designates very serious errors.
    Error = 4,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Trace => "Trace".fmt(f),
            Self::Debug => "Debug".fmt(f),
            Self::Info => "Info".fmt(f),
            Self::Warn => "Warn".fmt(f),
            Self::Error => "Error".fmt(f),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CallsiteKind {
    Event,
    Span,
}

impl std::fmt::Display for CallsiteKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Event => "Event".fmt(f),
            Self::Span => "Span".fmt(f),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DataEvent {
    pub callsite_id: CallsiteId,
    pub parent_span_id: Option<SpanId>,
    pub fields: FieldSet,
}

pub type FieldSet = Vec<(String, Value)>;

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

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(value) => format!("{:?}", value).fmt(f),
            Self::I64(value) => value.fmt(f),
            Self::U64(value) => value.fmt(f),
            Self::F64(value) => value.fmt(f),
            Self::Bool(value) => value.fmt(f),
            Self::Debug(value) => format!("{:?}", value).fmt(f),
            Self::Error {
                description,
                details,
            } => {
                write!(f, "Error: {}, {}", description, details)
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CallsiteId(pub u64);

impl std::fmt::Display for CallsiteId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        format!("{:016x}", self.0).fmt(f)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SpanId(pub u64);

impl std::fmt::Display for SpanId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        format!("{:016X}", self.0).fmt(f)
    }
}
