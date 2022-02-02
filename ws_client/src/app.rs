use eframe::{
    egui,
    epi::{self},
};

use ewebsock::{ws_connect, WsEvent, WsMessage, WsReceiver, WsSender};

#[derive(Default)]
pub struct WsClientApp {
    frontend: Option<FrontEnd>,
}

impl epi::App for WsClientApp {
    fn name(&self) -> &str {
        "eframe template"
    }

    fn setup(
        &mut self,
        _ctx: &egui::CtxRef,
        frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
        // let url = "ws://echo.websocket.events/.ws";
        let url = "ws://127.0.0.1:9002";

        // Make sure we wake up UI thread on event:
        let frame = frame.clone();
        let (ws_receiver, on_event) = WsReceiver::new(move || frame.request_repaint());

        let ws_sender = ws_connect(url.into(), on_event).unwrap();
        self.frontend = Some(FrontEnd::new(ws_sender, ws_receiver));
    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &epi::Frame) {
        if let Some(frontend) = &mut self.frontend {
            frontend.ui(ctx, frame);
        }
    }
}

// ----------------------------------------------------------------------------

struct FrontEnd {
    ws_sender: WsSender,
    ws_receiver: WsReceiver,
    events: Vec<WsEvent>,
    text_to_send: String,
}

impl FrontEnd {
    fn new(ws_sender: WsSender, ws_receiver: WsReceiver) -> Self {
        Self {
            ws_sender,
            ws_receiver,
            events: Default::default(),
            text_to_send: Default::default(),
        }
    }

    fn ui(&mut self, ctx: &egui::CtxRef, frame: &epi::Frame) {
        while let Some(event) = self.ws_receiver.try_recv() {
            self.events.push(event);
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.text_edit_singleline(&mut self.text_to_send).lost_focus()
                && ui.input().key_pressed(egui::Key::Enter)
            {
                self.ws_sender
                    .send(WsMessage::Text(std::mem::take(&mut self.text_to_send)));
            }

            ui.separator();
            ui.heading("Received messages:");
            for msg in &self.events {
                ui.label(format!("{:?}", msg));
            }
        });
    }
}
