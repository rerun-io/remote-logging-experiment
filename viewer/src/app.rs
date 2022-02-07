use eframe::{egui, epi};
use ewebsock::{ws_connect, WsEvent, WsMessage, WsReceiver, WsSender};
use rr_data::PubSubMsg;
use std::collections::HashMap;

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
    callsites: HashMap<rr_data::CallsiteId, rr_data::Callsite>,
    spans: HashMap<rr_data::SpanId, rr_data::Span>,
}

impl FrontEnd {
    fn new(ws_sender: WsSender, ws_receiver: WsReceiver) -> Self {
        Self {
            ws_sender,
            ws_receiver,
            lines: Default::default(),
            text_to_send: Default::default(),
            callsites: Default::default(),
            spans: Default::default(),
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
                                match &rr_msg.msg_enum {
                                    rr_data::MessageEnum::NewCallsite(callsite) => {
                                        self.callsites.insert(callsite.id, callsite.clone());
                                    }
                                    rr_data::MessageEnum::NewSpan(span) => {
                                        self.spans.insert(span.id, span.clone());
                                    }
                                    rr_data::MessageEnum::EnterSpan(_)
                                    | rr_data::MessageEnum::ExitSpan(_)
                                    | rr_data::MessageEnum::DataEvent(_) => {}
                                }
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

            ui.label("Hover to view call sites");
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for line in &self.lines {
                    match line {
                        Line::Text(text) => {
                            ui.label(text);
                        }
                        Line::Message(msg) => {
                            self.ui_msg(ui, msg);
                        }
                    }
                }
            });
        });
    }

    fn ui_msg(&self, ui: &mut egui::Ui, msg: &rr_data::Message) {
        let rr_data::Message { log_time, msg_enum } = msg;

        let time = format_time(log_time).unwrap_or_default();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(time).weak().monospace());
            self.ui_msg_enum(ui, msg_enum);
        });
    }

    fn ui_msg_enum(&self, ui: &mut egui::Ui, msg: &rr_data::MessageEnum) {
        match msg {
            rr_data::MessageEnum::NewCallsite(callsite) => {
                ui.label(format!("New callsite: {}", callsite.id))
                    .on_hover_ui(|ui| self.ui_callsite_id(ui, &callsite.id));
            }
            rr_data::MessageEnum::NewSpan(span) => {
                ui.label(format!("New span: {}", span.id))
                    .on_hover_ui(|ui| {
                        self.ui_span_id(ui, &span.id);
                        ui.separator();
                        ui.heading("Callsite:");
                        self.ui_callsite_id(ui, &span.callsite_id);
                    });
            }
            rr_data::MessageEnum::EnterSpan(span_id) => {
                ui.label(format!("Enter span: {}", span_id))
                    .on_hover_ui(|ui| {
                        self.ui_span_id(ui, span_id);
                    });
            }
            rr_data::MessageEnum::ExitSpan(span_id) => {
                ui.label(format!("Exit span: {}", span_id))
                    .on_hover_ui(|ui| {
                        self.ui_span_id(ui, span_id);
                    });
            }
            rr_data::MessageEnum::DataEvent(data_event) => self.ui_data_event(ui, data_event),
        }
    }

    fn ui_data_event(&self, ui: &mut egui::Ui, data_event: &rr_data::DataEvent) {
        let rr_data::DataEvent {
            callsite_id,
            parent_span_id,
            fields,
        } = data_event;

        let response = ui.horizontal(|ui| {
            for (key, value) in fields {
                ui.label(egui::RichText::new(format!("{}: ", key)).weak());
                ui.label(value.to_string());
            }
        });

        response
            .response
            .on_hover_ui(|ui| {
                ui.heading("Callsite:");
                self.ui_callsite_id(ui, callsite_id);
            })
            .on_hover_ui(|ui| {
                ui.heading("Parent span:");
                if let Some(parent_span_id) = parent_span_id {
                    self.ui_span_id(ui, parent_span_id);
                } else {
                    ui.label("<None>");
                }
            });
    }

    fn ui_callsite_id(&self, ui: &mut egui::Ui, callsite_id: &rr_data::CallsiteId) {
        if let Some(callsite) = self.callsites.get(callsite_id) {
            self.ui_callsite(ui, callsite);
        } else {
            ui.label(format!("Unknown callsite: {}", callsite_id)); // error
        }
    }

    fn ui_callsite(&self, ui: &mut egui::Ui, callsite: &rr_data::Callsite) {
        let rr_data::Callsite {
            id,
            kind,
            name,
            level,
            location,
            field_names,
        } = callsite;

        use itertools::Itertools as _;

        egui::Grid::new("callsite").num_columns(2).show(ui, |ui| {
            ui.label("Id:");
            ui.label(id.to_string());
            ui.end_row();

            ui.label("Kind:");
            ui.label(kind.to_string());
            ui.end_row();

            ui.label("Name:");
            ui.label(name.as_str());
            ui.end_row();

            ui.label("Level:");
            ui.label(level.to_string());
            ui.end_row();

            ui.label("Location:");
            ui.label(format_location(location));
            ui.end_row();

            ui.label("Field names:");
            ui.label(field_names.iter().join(" "));
            ui.end_row();
        });
    }

    fn ui_span_id(&self, ui: &mut egui::Ui, span_id: &rr_data::SpanId) {
        if let Some(span) = self.spans.get(span_id) {
            self.ui_span(ui, span);
        } else {
            ui.label(format!("Unknown span: {}", span_id)); // error
        }
    }

    fn ui_span(&self, ui: &mut egui::Ui, span: &rr_data::Span) {
        let rr_data::Span {
            id,
            parent_span_id,
            callsite_id,
        } = span;

        egui::Grid::new("callsite").num_columns(2).show(ui, |ui| {
            ui.label("Id:");
            ui.label(id.to_string());
            ui.end_row();

            ui.label("Parent span:");
            if let Some(parent_span_id) = parent_span_id {
                ui.label(parent_span_id.to_string())
                    .on_disabled_hover_ui(|ui| {
                        ui.heading("Parent span");
                        self.ui_span_id(ui, parent_span_id);
                    });
            } else {
                ui.label("<ROOT>");
            }
            ui.end_row();

            ui.label("Callsite:");
            ui.label(callsite_id.to_string())
                .on_disabled_hover_ui(|ui| {
                    self.ui_callsite_id(ui, callsite_id);
                });
            ui.end_row();
        });
    }
}

fn format_location(loc: &rr_data::Location) -> String {
    let rr_data::Location { module, file, line } = loc;
    match (file, line) {
        (None, None) => module.to_string(),
        (Some(file), None) => format!("{} {}", module, file),
        (None, Some(line)) => format!("{}, line {}", module, line),
        (Some(file), Some(line)) => format!("{} {}:{}", module, file, line),
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
        // TODO: assume relative time?
        None // `nanos_since_epoch` is likely not counting from epoch.
    }
}
