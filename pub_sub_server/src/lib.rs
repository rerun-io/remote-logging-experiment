use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use parking_lot::Mutex;
use rr_data::{PubSubMsg, TopicId, TopicMeta};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use std::{net::SocketAddr, ops::ControlFlow, time::Duration};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Error, WebSocketStream};

pub struct Topics {
    topics: Mutex<HashMap<TopicId, TopicStream>>,
    tx: tokio::sync::broadcast::Sender<Arc<rr_data::PubSubMsg>>,
}

impl Default for Topics {
    fn default() -> Self {
        let (tx, _rx) = tokio::sync::broadcast::channel(1024);
        Self {
            tx,
            topics: Default::default(),
        }
    }
}

#[derive(Clone)]
struct TopicStream {
    topic_meta: TopicMeta,
    messages: Vec<Arc<[u8]>>,
}

impl TopicStream {
    fn new(topic_meta: TopicMeta) -> Self {
        Self {
            topic_meta,
            messages: Default::default(),
        }
    }
}

// ----------------------------------------------------------------------------

pub struct Server {
    listener: TcpListener,
}

impl Server {
    /// Start a pub-sub server listening on the given port
    pub async fn new(port: u16) -> anyhow::Result<Self> {
        use anyhow::Context as _;

        let bind_addr = format!("127.0.0.1:{}", port);

        let listener = TcpListener::bind(&bind_addr)
            .await
            .context("Can't listen")?;
        eprintln!("Pub-sub listening on: {}", bind_addr);

        Ok(Self { listener })
    }

    /// Accept new connections forever
    pub async fn run(self) -> anyhow::Result<()> {
        use anyhow::Context as _;

        let topics = Arc::new(Topics::default());

        while let Ok((stream, _)) = self.listener.accept().await {
            let peer = stream
                .peer_addr()
                .context("connected streams should have a peer address")?;
            let topics = topics.clone();
            tokio::spawn(accept_connection(topics, peer, stream));
        }

        Ok(())
    }
}

async fn accept_connection(topics: Arc<Topics>, peer: SocketAddr, stream: TcpStream) {
    let span = tracing::span!(
        tracing::Level::INFO,
        "Connection",
        peer = peer.to_string().as_str()
    );
    let _enter = span.enter();
    tracing::info!("New WebSocket connection");

    if let Err(e) = handle_connection(&topics, stream).await {
        match e {
            Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => (),
            err => tracing::error!("Error processing connection: {}", err),
        }
    }
}

async fn handle_connection(topics: &Topics, stream: TcpStream) -> tungstenite::Result<()> {
    let ws_stream = accept_async(stream).await.expect("Failed to accept");
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let mut interval = tokio::time::interval(Duration::from_millis(1000));

    let mut broadcast_rx = topics.tx.subscribe();

    let mut subscribed_topics = HashSet::default();

    loop {
        tokio::select! {
            ws_msg = ws_receiver.next() => {
                match ws_msg {
                    Some(Ok(msg)) => {
                        if on_msg(&mut subscribed_topics, topics, &mut ws_sender, msg).await == ControlFlow::Break(()) {
                            break;
                        }
                    }
                    Some(Err(err)) => {
                        tracing::warn!("Error message: {:?}", err);
                        break;
                    }
                    None => {
                        break;
                    }
                }
            }
            pub_sub_msg = broadcast_rx.recv() => {
                let pub_sub_msg = pub_sub_msg.unwrap();
                let client_wants_msg = match &*pub_sub_msg {
                    rr_data::PubSubMsg::NewTopic(_) => {
                        true // Inform everyone about all new topics
                    }
                    rr_data::PubSubMsg::TopicMsg(topic_id, _) => {
                        subscribed_topics.contains(topic_id)
                    }
                    rr_data::PubSubMsg::SubscribeTo(_) | rr_data::PubSubMsg::ListTopics | rr_data::PubSubMsg::AllTopics(_) => {
                        unreachable!("Not broadcast")
                    }
                };
                if client_wants_msg {
                    tracing::debug!("Passing on message");
                    ws_sender.send(tungstenite::Message::Binary(pub_sub_msg.encode())).await?;
                }
            }
            _ = interval.tick() => {
                // ws_sender.send(Message::Text("tick".to_owned())).await?;
            }
        }
    }

    Ok(())
}

async fn on_msg(
    subscribed_topics: &mut HashSet<rr_data::TopicId>,
    topics: &Topics,
    ws_sender: &mut SplitSink<WebSocketStream<TcpStream>, tungstenite::Message>,
    msg: tungstenite::Message,
) -> ControlFlow<()> {
    match msg {
        tungstenite::Message::Text(text) => {
            tracing::warn!("Received unknown text message: {:?}", text);
            ControlFlow::Continue(())
        }
        tungstenite::Message::Binary(binary) => {
            if let Ok(pub_sub_msg) = rr_data::PubSubMsg::decode(&binary) {
                handle_pub_sub_msg(subscribed_topics, topics, ws_sender, pub_sub_msg).await
            } else {
                tracing::warn!("Received unknown binary message of length {}", binary.len());
                ControlFlow::Continue(())
            }
        }
        tungstenite::Message::Ping(binary) => {
            tracing::debug!("Received Pong");
            // respond with a Pong:
            if let Err(err) = ws_sender.send(tungstenite::Message::Ping(binary)).await {
                tracing::error!("Error sending: {:?}", err);
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        }
        tungstenite::Message::Pong(_binary) => {
            tracing::debug!("Received Pong");
            ControlFlow::Continue(())
        }
        tungstenite::Message::Close(close_frame) => {
            tracing::debug!("Received close: {:?}", close_frame);
            ControlFlow::Break(())
        }
    }
}

async fn handle_pub_sub_msg(
    subscribed_topics: &mut HashSet<rr_data::TopicId>,
    topics: &Topics,
    ws_sender: &mut SplitSink<WebSocketStream<TcpStream>, tungstenite::Message>,
    pub_sub_msg: rr_data::PubSubMsg,
) -> ControlFlow<()> {
    match &pub_sub_msg {
        rr_data::PubSubMsg::NewTopic(topic_meta) => {
            tracing::debug!("New topic: {:?}", topic_meta);
            let previous = topics
                .topics
                .lock()
                .insert(topic_meta.id, TopicStream::new(topic_meta.clone()));
            assert!(previous.is_none());
            topics.tx.send(pub_sub_msg.into()).unwrap(); // tell everyone about the new topic
        }
        rr_data::PubSubMsg::TopicMsg(topic_id, message) => {
            tracing::trace!("TopicMsg");
            if let Some(topic_stream) = topics.topics.lock().get_mut(topic_id) {
                topic_stream.messages.push(message.clone());
            }
            topics.tx.send(pub_sub_msg.into()).unwrap(); // tell everyone about the new message
        }
        rr_data::PubSubMsg::SubscribeTo(topic_id) => {
            tracing::debug!("Subscribing to {:?}", topic_id);
            let topic_stream = topics.topics.lock().get(topic_id).cloned();
            if let Some(topic_stream) = topic_stream {
                let messages = &topic_stream.messages;

                tracing::debug!("Sending a backlog of {} messages", messages.len());
                for message in messages {
                    let pub_sub_msg = PubSubMsg::TopicMsg(*topic_id, message.clone());
                    if let Err(err) = ws_sender
                        .send(tungstenite::Message::Binary(pub_sub_msg.encode()))
                        .await
                    {
                        tracing::error!("Error sending: {:?}", err);
                        return ControlFlow::Break(());
                    }
                }
            }
            subscribed_topics.insert(*topic_id);
        }
        rr_data::PubSubMsg::ListTopics => {
            tracing::debug!("ListTopics");
            let all_topic_metas = topics
                .topics
                .lock()
                .values()
                .map(|ts| ts.topic_meta.clone())
                .collect();
            let pub_sub_msg = PubSubMsg::AllTopics(all_topic_metas);
            if let Err(err) = ws_sender
                .send(tungstenite::Message::Binary(pub_sub_msg.encode()))
                .await
            {
                tracing::error!("Error sending: {:?}", err);
                return ControlFlow::Break(());
            }
        }
        rr_data::PubSubMsg::AllTopics(_) => {
            tracing::debug!("Client sent AllTopics message. Weird");
        }
    }
    ControlFlow::Continue(())
}
