use eframe::{egui, epi};
use ewebsock::{ws_connect, WsEvent, WsMessage, WsReceiver, WsSender};
use rr_data::PubSubMsg;

#[derive(Default)]
pub struct WsClientApp {
    frontend: Option<FrontEnd>,
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
            WsReceiver::new_with_callback(move || frame.request_repaint());

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

enum Line {
    Text(String),
    Message(rr_data::Message),
}

// ----------------------------------------------------------------------------

struct FrontEnd {
    ws_sender: WsSender,
    ws_receiver: WsReceiver,
    lines: Vec<Line>,
    text_to_send: String,
}

impl FrontEnd {
    fn new(ws_sender: WsSender, ws_receiver: WsReceiver) -> Self {
        Self {
            ws_sender,
            ws_receiver,
            lines: Default::default(),
            text_to_send: Default::default(),
        }
    }

    fn ui(&mut self, ctx: &egui::CtxRef, frame: &epi::Frame) {
        while let Some(event) = self.ws_receiver.try_recv() {
            if let WsEvent::Message(WsMessage::Binary(payload)) = &event {
                if let Ok(pub_sub_msg) = rr_data::PubSubMsg::decode(payload) {
                    match pub_sub_msg {
                        PubSubMsg::NewTopic(topic_id, topic_meta) => {
                            self.lines.push(Line::Text(format!(
                                "Subscribing to new topic: {:?}",
                                topic_meta
                            )));
                            self.ws_sender
                                .send(WsMessage::Binary(PubSubMsg::SubscribeTo(topic_id).encode()));
                            continue;
                        }
                        PubSubMsg::TopicMsg(_topic_id, payload) => {
                            if let Ok(rr_msg) = rr_data::Message::decode(&payload) {
                                self.lines.push(Line::Message(rr_msg));
                                continue;
                            }
                        }
                        PubSubMsg::SubscribeTo(_) => {
                            // weird
                        }
                    }
                    continue;
                }
            }
            self.lines.push(Line::Text(format!("Recevied {:?}", event)));
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
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
            egui::ScrollArea::vertical().show(ui, |ui| {
                for line in &self.lines {
                    match line {
                        Line::Text(text) => {
                            ui.label(text);
                        }
                        Line::Message(msg) => {
                            ui_msg(ui, msg);
                        }
                    }
                }
            });
        });
    }
}

fn ui_msg(ui: &mut egui::Ui, msg: &rr_data::Message) {
    let rr_data::Message { log_time, msg_enum } = msg;

    let time = format_time(log_time).unwrap_or_default();
    ui.horizontal(|ui| {
        ui.monospace(time);
        ui_msg_enum(ui, msg_enum);
    });
}

fn ui_msg_enum(ui: &mut egui::Ui, msg: &rr_data::MessageEnum) {
    match msg {
        rr_data::MessageEnum::DataEvent(data_event) => ui_data_event(ui, data_event),
    }
}

fn ui_data_event(ui: &mut egui::Ui, data_event: &rr_data::DataEvent) {
    let rr_data::DataEvent { callsite, fields } = data_event;
    // TODO: look up callsite, at least on hover
    for (key, value) in fields {
        ui.label(egui::RichText::new(format!("{}: ", key)).weak());
        ui.label(value.to_string());
    }
}

fn format_time(time: &rr_data::Time) -> Option<String> {
    let nanos_since_epoch = time.nanos_since_epoch();
    let years_since_epoch = nanos_since_epoch / 1_000_000_000 / 60 / 60 / 24 / 365;
    if 50 <= years_since_epoch && years_since_epoch <= 150 {
        use chrono::TimeZone as _;
        let datetime = chrono::Utc.timestamp(
            nanos_since_epoch / 1_000_000_000,
            (nanos_since_epoch % 1_000_000_000) as _,
        );
        Some(datetime.format("%Y-%m-%d %H:%M:%S%.3f UTC").to_string())
    } else {
        None // `nanos_since_epoch` is likely not counting from epoch.
    }
}
