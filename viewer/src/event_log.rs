use eframe::egui;
use std::collections::HashMap;

enum Line {
    Text(String),
    Message(rr_data::Message),
}

// ----------------------------------------------------------------------------

#[derive(Default)]
pub struct EventLog {
    callsites: HashMap<rr_data::CallsiteId, rr_data::Callsite>,
    spans: HashMap<rr_data::SpanId, rr_data::Span>,
    lines: Vec<Line>,
}

impl EventLog {
    pub fn on_message(&mut self, msg: rr_data::Message) {
        match &msg.msg_enum {
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
        self.lines.push(Line::Message(msg));
    }

    pub fn on_text(&mut self, text: String) {
        self.lines.push(Line::Text(text));
    }

    pub fn ui(&self, ui: &mut egui::Ui) {
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
    }

    fn ui_msg(&self, ui: &mut egui::Ui, msg: &rr_data::Message) {
        let rr_data::Message { log_time, msg_enum } = msg;

        let time = crate::misc::format_time(log_time);
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
            crate::misc::ui_callsite(ui, callsite);
        } else {
            ui.label(format!("Unknown callsite: {}", callsite_id)); // error
        }
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
