use eframe::{egui, epi};

use crate::{ws_connect, WsEvent, WsMessage, WsReceiver, WsSender};

#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct WsClientApp {
    #[cfg_attr(feature = "persistence", serde(skip))]
    ws_receiver: WsReceiver,

    #[cfg_attr(feature = "persistence", serde(skip))]
    ws_sender: WsSender,

    #[cfg_attr(feature = "persistence", serde(skip))]
    messages: Vec<WsEvent>,

    #[cfg_attr(feature = "persistence", serde(skip))]
    text_to_send: String,
}

impl Default for WsClientApp {
    fn default() -> Self {
        let (ws_receiver, ws_sender) = ws_connect("ws://echo.websocket.events/.ws".into()).unwrap();

        Self {
            ws_receiver,
            ws_sender,
            messages: Default::default(),
            text_to_send: Default::default(),
        }
    }
}

impl epi::App for WsClientApp {
    fn name(&self) -> &str {
        "eframe template"
    }

    fn setup(
        &mut self,
        _ctx: &egui::CtxRef,
        _frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
        #[cfg(feature = "persistence")]
        if let Some(storage) = _storage {
            *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
        }
    }

    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &epi::Frame) {
        while let Some(msg) = self.ws_receiver.try_recv() {
            self.messages.push(msg);
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
