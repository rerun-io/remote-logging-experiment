use crate::{Result, WsEvent, WsMessage};

pub struct WsSender {
    sender: websocket::sender::Writer<websocket::sync::stream::TcpStream>,
}

impl WsSender {
    pub fn send(&mut self, msg: WsMessage) -> Result<()> {
        let result = match msg {
            WsMessage::Binary(data) => self
                .sender
                .send_message(&websocket::OwnedMessage::Binary(data)),
            WsMessage::Text(text) => self
                .sender
                .send_message(&websocket::OwnedMessage::Text(text)),
            unknown => {
                panic!("Don't know how to send message: {:?}", unknown);
            }
        };
        result.map_err(|err| err.to_string())
    }
}

pub struct WsReceiver {
    rx: std::sync::mpsc::Receiver<WsEvent>,
}

impl WsReceiver {
    pub fn try_recv(&self) -> Option<WsEvent> {
        self.rx.try_recv().ok()
    }
}

pub fn ws_connect(url: String) -> Result<(WsReceiver, WsSender)> {
    let client = websocket::ClientBuilder::new(&url)
        .map_err(|err| err.to_string())?
        .connect_insecure()
        .map_err(|err| err.to_string())?;

    let (mut reader, sender) = client.split().map_err(|err| err.to_string())?;

    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::Builder::new()
        .name("websocket_receiver".into())
        .spawn(move || {
            loop {
                match reader.recv_message() {
                    Ok(message) => {
                        let msg = match message {
                            websocket::OwnedMessage::Binary(binary) => WsMessage::Binary(binary),
                            websocket::OwnedMessage::Text(text) => WsMessage::Text(text),
                            websocket::OwnedMessage::Close(close_data) => {
                                eprintln!("Websocket closed: {:#?}", close_data);
                                break;
                            }
                            websocket::OwnedMessage::Ping(data) => WsMessage::Ping(data),
                            websocket::OwnedMessage::Pong(data) => WsMessage::Pong(data),
                        };
                        if tx.send(WsEvent::Message(msg)).is_err() {
                            break;
                        }
                    }
                    Err(err) => {
                        eprintln!("Websocket error: {:#?}", err);
                    }
                }
            }
            eprintln!("Stopping websocket receiver thread")
        })
        .unwrap();

    Ok((WsReceiver { rx }, WsSender { sender }))
}
