use crate::{Result, WsEvent, WsMessage};

macro_rules! console_log {
    ($($t:tt)*) => (web_sys::console::log_1(&format!($($t)*).into()))
}

fn string_from_js_value(s: wasm_bindgen::JsValue) -> String {
    s.as_string().unwrap_or(format!("{:#?}", s))
}

fn string_from_js_string(s: js_sys::JsString) -> String {
    s.as_string().unwrap_or(format!("{:#?}", s))
}

#[derive(Clone)]
pub struct WsSender {
    ws: web_sys::WebSocket,
}

impl WsSender {
    pub fn send(&mut self, msg: WsMessage) -> Result<()> {
        let result = match msg {
            WsMessage::Binary(data) => {
                self.ws.set_binary_type(web_sys::BinaryType::Blob);
                self.ws.send_with_u8_array(&data)
            }
            WsMessage::Text(text) => self.ws.send_with_str(&text),
            unknown => {
                panic!("Don't know how to send message: {:?}", unknown);
            }
        };
        result.map_err(string_from_js_value)
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
    // Based on https://rustwasm.github.io/wasm-bindgen/examples/websockets.html

    console_log!("spawn_ws_client");
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast as _;

    let (tx, rx) = std::sync::mpsc::channel();

    // Connect to an server
    let ws = web_sys::WebSocket::new(&url).map_err(string_from_js_value)?;

    // For small binary messages, like CBOR, Arraybuffer is more efficient than Blob handling
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

    // onmessage callback
    {
        let tx = tx.clone();
        let onmessage_callback = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
            // Handle difference Text/Binary,...
            if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                console_log!("message event, received arraybuffer: {:?}", abuf);
                let array = js_sys::Uint8Array::new(&abuf);
                let len = array.byte_length() as usize;
                console_log!("Arraybuffer received {} bytes: {:?}", len, array.to_vec());
                tx.send(WsEvent::Message(WsMessage::Binary(array.to_vec())))
                    .unwrap(); // TODO: error handling
            } else if let Ok(blob) = e.data().dyn_into::<web_sys::Blob>() {
                console_log!("message event, received blob: {:?}", blob);
                // better alternative to juggling with FileReader is to use https://crates.io/crates/gloo-file
                let fr = web_sys::FileReader::new().unwrap();
                let fr_c = fr.clone();
                // create onLoadEnd callback
                let tx = tx.clone();
                let onloadend_cb = Closure::wrap(Box::new(move |_e: web_sys::ProgressEvent| {
                    let array = js_sys::Uint8Array::new(&fr_c.result().unwrap());
                    let len = array.byte_length() as usize;
                    console_log!("Blob received {} bytes: {:?}", len, array.to_vec());
                    tx.send(WsEvent::Message(WsMessage::Binary(array.to_vec())))
                        .unwrap(); // TODO: error handling
                })
                    as Box<dyn FnMut(web_sys::ProgressEvent)>);
                fr.set_onloadend(Some(onloadend_cb.as_ref().unchecked_ref()));
                fr.read_as_array_buffer(&blob).expect("blob not readable");
                onloadend_cb.forget();
            } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                console_log!("message event, received Text: {:?}", txt);
                tx.send(WsEvent::Message(WsMessage::Text(string_from_js_string(
                    txt,
                ))))
                .unwrap(); // TODO: error handling
            } else {
                console_log!("message event, received Unknown: {:?}", e.data());
                tx.send(WsEvent::Message(WsMessage::Unknown(string_from_js_value(
                    e.data(),
                ))))
                .unwrap(); // TODO: error handling
            }
        }) as Box<dyn FnMut(web_sys::MessageEvent)>);

        // set message event handler on WebSocket
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));

        // forget the callback to keep it alive
        onmessage_callback.forget();
    }

    {
        let tx = tx.clone();
        let onerror_callback = Closure::wrap(Box::new(move |error_event: web_sys::ErrorEvent| {
            console_log!("error event: {:?}", error_event);
            tx.send(WsEvent::Error(error_event.message())).unwrap(); // TODO: error handling
        }) as Box<dyn FnMut(web_sys::ErrorEvent)>);
        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();
    }

    {
        let onopen_callback = Closure::wrap(Box::new(move |_| {
            console_log!("socket opened");
            tx.send(WsEvent::Opened).unwrap(); // TODO: error handling
        }) as Box<dyn FnMut(wasm_bindgen::JsValue)>);
        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();
    }

    Ok((WsReceiver { rx }, WsSender { ws }))
}
