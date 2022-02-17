use crate::viewer::Viewer;
use eframe::{egui, epi};

#[derive(Default)]
pub struct WsClientApp {
    pubsub_url: String,
    frontend: Option<Viewer>,
}

impl epi::App for WsClientApp {
    fn name(&self) -> &str {
        "Live Log Viewer"
    }

    fn setup(
        &mut self,
        _ctx: &egui::Context,
        frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
        if let Some(web_info) = &frame.info().web_info {
            if let Some(pubsub_url) = web_info.location.query_map.get("pubsub") {
                self.pubsub_url = pubsub_url.clone()
            }
        }
        if self.pubsub_url.is_empty() {
            // self.pubsub_url = "ws://echo.websocket.lines/.ws";
            self.pubsub_url = format!("ws://127.0.0.1:{}", rr_data::DEFAULT_PUB_SUB_PORT);
        }

        self.connect(frame.clone());
    }

    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        egui::TopBottomPanel::top("server").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("URL:");
                if ui.text_edit_singleline(&mut self.pubsub_url).lost_focus()
                    && ui.input().key_pressed(egui::Key::Enter)
                {
                    self.connect(frame.clone());
                }
            });
        });

        if let Some(frontend) = &mut self.frontend {
            frontend.ui(ctx);
        }
    }
}

impl WsClientApp {
    fn connect(&mut self, frame: epi::Frame) {
        // Make sure we wake up UI thread on event:
        let (ws_receiver, on_event) =
            ewebsock::WsReceiver::new_with_callback(move || frame.request_repaint());

        let ws_sender = ewebsock::ws_connect(self.pubsub_url.clone(), on_event).unwrap();
        self.frontend = Some(Viewer::new(ws_sender, ws_receiver));
    }
}
