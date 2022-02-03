use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use std::{net::SocketAddr, ops::ControlFlow, time::Duration};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Error, WebSocketStream};
use tungstenite::{Message, Result};

// pub struct Topic {
// }

// pub struct Broadcaster {
//     tx: tokio::sync::broadcast::Sender<Arc[u8]>,
// }

// impl Broadcaster {
//     fn new() -> Self {
//         let (tx, _rx) = tokio::sync::broadcast::channel(1024);
//         Self { tx }
//     }
// }

async fn accept_connection(peer: SocketAddr, stream: TcpStream) {
    if let Err(e) = handle_connection(peer, stream).await {
        match e {
            Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => (),
            err => tracing::error!("Error processing connection: {}", err),
        }
    }
}

async fn handle_connection(peer: SocketAddr, stream: TcpStream) -> Result<()> {
    let span = tracing::span!(
        tracing::Level::INFO,
        "Connection",
        peer = peer.to_string().as_str()
    );
    let _enter = span.enter();
    tracing::info!("New WebSocket connection");

    let ws_stream = accept_async(stream).await.expect("Failed to accept");
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let mut interval = tokio::time::interval(Duration::from_millis(1000));

    // Echo incoming WebSocket messages and send a message periodically every second.

    loop {
        tokio::select! {
            msg = ws_receiver.next() => {
                match msg {
                    Some(Ok(msg)) => {
                        if on_msg(&mut ws_sender, msg).await == ControlFlow::Break(()) {
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
            _ = interval.tick() => {
                ws_sender.send(Message::Text("tick".to_owned())).await?;
            }
        }
    }

    Ok(())
}

async fn on_msg(
    ws_sender: &mut SplitSink<WebSocketStream<TcpStream>, Message>,
    msg: Message,
) -> ControlFlow<()> {
    tracing::info!("Message received");
    if let Message::Binary(binary) = &msg {
        if let Ok(rr_msg) = rr_data::Message::decode(binary) {
            tracing::info!("Received a message:\n{:#?}\n", rr_msg);
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

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let addr = "127.0.0.1:9002";
    let listener = TcpListener::bind(&addr).await.expect("Can't listen");
    tracing::info!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let peer = stream
            .peer_addr()
            .expect("connected streams should have a peer address");
        tokio::spawn(accept_connection(peer, stream));
    }
}
