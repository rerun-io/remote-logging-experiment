use parking_lot::Mutex;
use rr_data::PubSubMsg;
use std::sync::Arc;

struct RrConnection {
    send: ewebsock::WsSender,
    _recv: ewebsock::WsReceiver,
}

impl RrConnection {
    fn to_ws_server(url: String) -> Self {
        let (send, _recv) = ewebsock::connect(url).unwrap();
        Self { send, _recv }
    }

    fn send(&mut self, msg: rr_data::PubSubMsg) {
        self.send.send(ewebsock::WsMessage::Binary(msg.encode()));
    }
}

// ----------------------------------------------------------------------------

pub struct RrLogger {
    topic_id: rr_data::TopicId,
    connection: Arc<Mutex<RrConnection>>,
}

// static_assertions::assert_impl_all!(RrLogger: Send, Sync);

impl RrLogger {
    pub fn to_ws_server(url: String, topic_meta: rr_data::TopicMeta) -> Self {
        let topic_id = rr_data::TopicId::random();
        let mut connection = RrConnection::to_ws_server(url);
        connection.send(PubSubMsg::NewTopic(topic_id, topic_meta));
        Self {
            topic_id,
            connection: Arc::new(Mutex::new(connection)),
        }
    }

    pub fn send(&self, msg: rr_data::Message) {
        let msg = rr_data::PubSubMsg::TopicMsg(self.topic_id, msg.encode());
        self.connection.lock().send(msg);
    }
}

impl<S: tracing::Subscriber> tracing_subscriber::layer::Layer<S> for RrLogger {
    fn on_layer(&mut self, _subscriber: &mut S) {
        eprintln!("\non_layer");
    }

    fn register_callsite(
        &self,
        metadata: &'static tracing::Metadata<'static>,
    ) -> tracing::subscriber::Interest {
        let kind = if metadata.is_event() {
            rr_data::CallsiteKind::Event
        } else {
            rr_data::CallsiteKind::Span
        };

        let level = if *metadata.level() == tracing::Level::ERROR {
            rr_data::LogLevel::Error
        } else if *metadata.level() == tracing::Level::WARN {
            rr_data::LogLevel::Warn
        } else if *metadata.level() == tracing::Level::INFO {
            rr_data::LogLevel::Info
        } else if *metadata.level() == tracing::Level::DEBUG {
            rr_data::LogLevel::Debug
        } else {
            rr_data::LogLevel::Trace
        };

        let field_names = metadata
            .fields()
            .iter()
            .map(|field| field.name().to_owned())
            .collect();

        let location = rr_data::Location {
            module: metadata.target().to_string(),
            file: metadata.file().map(|t| t.to_string()),
            line: metadata.line(),
        };

        let rr_callsite = rr_data::Callsite {
            id: to_callsite_id(&metadata.callsite()),
            kind,
            name: metadata.name().to_string(),
            level,
            location,
            field_names,
        };
        self.send(rr_data::Message::now(rr_data::MessageEnum::NewCallsite(
            rr_callsite,
        )));

        tracing::subscriber::Interest::always()
    }

    fn enabled(
        &self,
        _metadata: &tracing::Metadata<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        true
    }

    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let parent_span_id = attrs
            .parent()
            .map(to_span_id)
            .or_else(|| ctx.current_span().id().map(to_span_id));

        let rr_span = rr_data::Span {
            id: to_span_id(id),
            callsite_id: to_callsite_id(&attrs.metadata().callsite()),
            parent_span_id,
        };
        self.send(rr_data::Message::now(rr_data::MessageEnum::NewSpan(
            rr_span,
        )));
    }

    fn on_record(
        &self,
        span: &tracing::Id,
        values: &tracing::span::Record<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        eprintln!("\non_record: {:?} {:?}", span, values);
    }

    fn on_follows_from(
        &self,
        span: &tracing::Id,
        follows: &tracing::Id,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        eprintln!("\non_follows_from: {:?} follows {:?}", span, follows);
    }

    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let parent_span_id = event
            .parent()
            .map(to_span_id)
            .or_else(|| ctx.current_span().id().map(to_span_id));

        let mut kv_collector = KvCollector::default();
        event.record(&mut kv_collector);

        let rr_event = rr_data::DataEvent {
            callsite_id: to_callsite_id(&event.metadata().callsite()),
            parent_span_id,
            fields: kv_collector.values,
        };

        self.send(rr_data::Message::now(rr_data::MessageEnum::DataEvent(
            rr_event,
        )));
    }

    fn on_enter(&self, id: &tracing::Id, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        self.send(rr_data::Message::now(rr_data::MessageEnum::EnterSpan(
            to_span_id(id),
        )));
    }

    fn on_exit(&self, id: &tracing::Id, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        self.send(rr_data::Message::now(rr_data::MessageEnum::ExitSpan(
            to_span_id(id),
        )));
    }

    fn on_close(&self, id: tracing::Id, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        eprintln!("\non_close: {:?}", id);
    }

    fn on_id_change(
        &self,
        old: &tracing::Id,
        new: &tracing::Id,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        eprintln!("\non_id_change: {:?} {:?}", old, new);
    }
}

#[derive(Default)]
struct KvCollector {
    pub values: Vec<(String, rr_data::Value)>,
}

impl tracing::field::Visit for KvCollector {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let value = rr_data::Value::Debug(format!("{:#?}", value));
        self.values.push((field.name().to_owned(), value));
    }
    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        let value = rr_data::Value::F64(value);
        self.values.push((field.name().to_owned(), value));
    }
    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        let value = rr_data::Value::I64(value);
        self.values.push((field.name().to_owned(), value));
    }
    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        let value = rr_data::Value::U64(value);
        self.values.push((field.name().to_owned(), value));
    }
    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        let value = rr_data::Value::Bool(value);
        self.values.push((field.name().to_owned(), value));
    }
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        let value = rr_data::Value::String(value.to_owned());
        self.values.push((field.name().to_owned(), value));
    }
    fn record_error(
        &mut self,
        field: &tracing::field::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        let value = rr_data::Value::Error {
            description: value.to_string(),
            details: format!("{:#?}", value),
        };
        self.values.push((field.name().to_owned(), value));
    }
}

/// Hash the given value with a predictable hasher.
#[inline]
pub fn hash(value: impl std::hash::Hash) -> u64 {
    use std::hash::Hasher as _;
    let mut hasher = wyhash::WyHash::default();
    value.hash(&mut hasher);
    hasher.finish()
}

fn to_callsite_id(id: &tracing::callsite::Identifier) -> rr_data::CallsiteId {
    rr_data::CallsiteId(hash(id))
}

fn to_span_id(id: &tracing::Id) -> rr_data::SpanId {
    rr_data::SpanId(hash(id))
}
