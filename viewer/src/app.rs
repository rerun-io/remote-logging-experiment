use crate::viewer::Viewer;
use eframe::{egui, epi};

#[derive(Default)]
pub struct WsClientApp {
    frontend: Option<Viewer>,
}

impl epi::App for WsClientApp {
    fn name(&self) -> &str {
        "Live Log Viewer"
    }

    fn setup(
        &mut self,
        _ctx: &egui::CtxRef,
        frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
        // let url = "ws://echo.websocket.lines/.ws";
        let url = "ws://127.0.0.1:9002";

        // Make sure we wake up UI thread on event:
        let frame = frame.clone();
        let (ws_receiver, on_event) =
            ewebsock::WsReceiver::new_with_callback(move || frame.request_repaint());

        let ws_sender = ewebsock::ws_connect(url.into(), on_event).unwrap();
        self.frontend = Some(Viewer::new(ws_sender, ws_receiver));
    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &epi::Frame) {
        if let Some(frontend) = &mut self.frontend {
            frontend.ui(ctx, frame);
        }
    }
}
