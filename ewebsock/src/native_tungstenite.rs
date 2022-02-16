use crate::{EventHandler, Result, WsEvent, WsMessage};

pub struct WsSender {
    tx: tokio::sync::mpsc::Sender<WsMessage>,
}

impl WsSender {
    pub fn send(&mut self, msg: WsMessage) {
        let tx = self.tx.clone();
        tokio::spawn(async move { tx.send(msg).await });
    }
}

pub async fn ws_connect_async(
    url: String,
    outgoing_messages_stream: impl futures::Stream<Item = WsMessage>,
    on_event: EventHandler,
) -> Result<()> {
    use futures::StreamExt as _;

    let (ws_stream, _) = tokio_tungstenite::connect_async(url)
        .await
        .map_err(|err| err.to_string())?;

    tracing::info!("WebSocket handshake has been successfully completed");
    on_event(WsEvent::Opened);

    let (write, read) = ws_stream.split();

    let writer = outgoing_messages_stream
        .map(|ws_message| match ws_message {
            WsMessage::Text(text) => tungstenite::protocol::Message::Text(text),
            WsMessage::Binary(data) => tungstenite::protocol::Message::Binary(data),
            WsMessage::Ping(data) => tungstenite::protocol::Message::Ping(data),
            WsMessage::Pong(data) => tungstenite::protocol::Message::Pong(data),
            WsMessage::Unknown(_) => unimplemented!(),
        })
        .map(Ok)
        .forward(write);

    let reader = read.for_each(move |event| {
        let ws_event = match event {
            Ok(message) => match message {
                tungstenite::protocol::Message::Text(text) => {
                    WsEvent::Message(WsMessage::Text(text))
                }
                tungstenite::protocol::Message::Binary(data) => {
                    WsEvent::Message(WsMessage::Binary(data))
                }
                tungstenite::protocol::Message::Ping(data) => {
                    WsEvent::Message(WsMessage::Ping(data))
                }
                tungstenite::protocol::Message::Pong(data) => {
                    WsEvent::Message(WsMessage::Pong(data))
                }
                tungstenite::protocol::Message::Close(_) => WsEvent::Closed,
            },
            Err(err) => WsEvent::Error(err.to_string()),
        };
        on_event(ws_event);
        async {}
    });

    futures_util::pin_mut!(reader, writer);
    futures_util::future::select(reader, writer).await;

    Ok(())
}

pub fn ws_connect(url: String, on_event: EventHandler) -> Result<WsSender> {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1000);

    let outgoing_messages_stream = async_stream::stream! {
        while let Some(item) = rx.recv().await {
            yield item;
        }
    };

    tokio::spawn(async move { ws_connect_async(url, outgoing_messages_stream, on_event).await });
    Ok(WsSender { tx })
}
