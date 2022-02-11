use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use std::{collections::HashSet, sync::Arc};
use std::{net::SocketAddr, ops::ControlFlow, time::Duration};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Error, WebSocketStream};

/// Start a pub-sub server listening on the given port
pub async fn run(bind_addr: &str) -> anyhow::Result<()> {
    use anyhow::Context as _;

    let listener = TcpListener::bind(bind_addr).await.context("Can't listen")?;
    tracing::info!("Listening on: {}", bind_addr);

    let broadcaster = Arc::new(Broadcaster::new());

    while let Ok((stream, _)) = listener.accept().await {
        let peer = stream
            .peer_addr()
            .context("connected streams should have a peer address")?;
        let broadcaster = broadcaster.clone();
        tokio::spawn(accept_connection(broadcaster, peer, stream));
    }

    Ok(())
}

pub struct Broadcaster {
    tx: tokio::sync::broadcast::Sender<Arc<rr_data::PubSubMsg>>,
}

impl Broadcaster {
    fn new() -> Self {
        let (tx, _rx) = tokio::sync::broadcast::channel(1024);
        Self { tx }
    }
}

async fn accept_connection(broadcaster: Arc<Broadcaster>, peer: SocketAddr, stream: TcpStream) {
    let span = tracing::span!(
        tracing::Level::INFO,
        "Connection",
        peer = peer.to_string().as_str()
    );
    let _enter = span.enter();
    tracing::info!("New WebSocket connection");

    if let Err(e) = handle_connection(&broadcaster, stream).await {
        match e {
            Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => (),
            err => tracing::error!("Error processing connection: {}", err),
        }
    }
}

async fn handle_connection(
    broadcaster: &Broadcaster,
    stream: TcpStream,
) -> tungstenite::Result<()> {
    let ws_stream = accept_async(stream).await.expect("Failed to accept");
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let mut interval = tokio::time::interval(Duration::from_millis(1000));

    let mut pub_sub_rx = broadcaster.tx.subscribe();

    let mut subscribed_topics = HashSet::default();

    loop {
        tokio::select! {
            ws_msg = ws_receiver.next() => {
                match ws_msg {
                    Some(Ok(msg)) => {
                        if on_msg(&mut subscribed_topics, broadcaster, &mut ws_sender, msg).await == ControlFlow::Break(()) {
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
            pub_sub_msg = pub_sub_rx.recv() => {
                let pub_sub_msg = pub_sub_msg.unwrap();
                let should_send = match &*pub_sub_msg {
                    rr_data::PubSubMsg::NewTopic(_, _) => {
                        true // Inform everyone about all new topics
                    }
                    rr_data::PubSubMsg::TopicMsg(topic_id, _) => {
                        subscribed_topics.contains(topic_id)
                    }
                    rr_data::PubSubMsg::SubscribeTo(_) => {
                        false // clients don't care what topics other clients subscribe to.
                    }
                };
                if should_send {
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
    broadcaster: &Broadcaster,
    ws_sender: &mut SplitSink<WebSocketStream<TcpStream>, tungstenite::Message>,
    msg: tungstenite::Message,
) -> ControlFlow<()> {
    tracing::debug!("Message received");
    if let tungstenite::Message::Binary(binary) = &msg {
        if let Ok(pub_sub_msg) = rr_data::PubSubMsg::decode(binary) {
            if let rr_data::PubSubMsg::SubscribeTo(topic_id) = pub_sub_msg {
                subscribed_topics.insert(topic_id);
            } else {
                broadcaster.tx.send(pub_sub_msg.into()).unwrap();
            }
        }
    }

    if msg.is_text() || msg.is_binary() {
        if let Err(err) = ws_sender.send(msg).await {
            tracing::error!("Error sending: {:?}", err);
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    } else if msg.is_close() {
        ControlFlow::Break(())
    } else {
        ControlFlow::Continue(())
    }
}
