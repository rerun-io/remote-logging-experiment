use eframe::{egui, epi};

use ewebsock::{ws_connect, WsEvent, WsMessage, WsReceiver, WsSender};

pub struct WsClientApp {
    ws_receiver: WsReceiver,
    frontend: FrontEnd,
}

impl Default for WsClientApp {
    fn default() -> Self {
        // let (ws_receiver, ws_sender) = ws_connect("ws://echo.websocket.events/.ws".into()).unwrap();
        let (ws_receiver, ws_sender) = ws_connect("ws://127.0.0.1:9002".into()).unwrap();

        Self {
            ws_receiver,
            frontend: FrontEnd::new(ws_sender),
        }
    }
}

impl epi::App for WsClientApp {
    fn name(&self) -> &str {
        "eframe template"
    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &epi::Frame) {
        while let Some(msg) = self.ws_receiver.try_recv() {
            self.frontend.on_ws_event(msg);
        }

        self.frontend.ui(ctx, frame);
    }
}

// ----------------------------------------------------------------------------

struct FrontEnd {
    ws_sender: WsSender,
    messages: Vec<WsEvent>,
    text_to_send: String,
}

impl FrontEnd {
    fn new(ws_sender: WsSender) -> Self {
        Self {
            ws_sender,
            messages: Default::default(),
            text_to_send: Default::default(),
        }
    }

    fn on_ws_event(&mut self, msg: WsEvent) {
        self.messages.push(msg);
    }

    fn ui(&mut self, ctx: &egui::CtxRef, frame: &epi::Frame) {
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
                    .send(WsMessage::Text(std::mem::take(&mut self.text_to_send)))
                    .unwrap();
            }

            ui.separator();
            ui.heading("Received messages:");
            for msg in &self.messages {
                ui.label(format!("{:?}", msg));
            }
        });
    }
}
