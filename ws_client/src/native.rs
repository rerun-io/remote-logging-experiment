use crate::{Result, WsMessage};

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
            WsMessage::Unknown(_) => {
                panic!();
            }
        };
        result.map_err(|err| err.to_string())
    }
}

pub struct WsReceiver {
    reader: websocket::receiver::Reader<websocket::sync::stream::TcpStream>,
}

pub fn ws_connect(url: String) -> Result<(WsReceiver, WsSender)> {
    let client = websocket::ClientBuilder::new(&url)
        .unwrap()
        .connect_insecure()
        .unwrap();
    let (reader, sender) = client.split().unwrap();
    Ok((WsReceiver { reader }, WsSender { sender }))
}
