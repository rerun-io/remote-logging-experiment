use eframe::egui;

enum Line {
    Text(String),
    Message(rr_data::Message),
}

// ----------------------------------------------------------------------------

/// View every event and meta-event
#[derive(Default)]
pub struct EventLog {
    span_tree: crate::span_tree::SpanTree,
    lines: Vec<Line>,
}

impl EventLog {
    pub fn on_message(&mut self, msg: rr_data::Message) {
        self.span_tree.on_mesage(&msg);
        self.lines.push(Line::Message(msg));
    }

    pub fn on_text(&mut self, text: String) {
        self.lines.push(Line::Text(text));
    }

    pub fn ui(&self, ui: &mut egui::Ui) {
        ui.label("All the events that the viewer receives");
        ui.label("Hover to view call sites");
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
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

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(log_time.format()).weak().monospace());
            self.ui_msg_enum(ui, msg_enum);
        });
    }

    fn ui_msg_enum(&self, ui: &mut egui::Ui, msg: &rr_data::MessageEnum) {
        match msg {
            rr_data::MessageEnum::NewCallsite(callsite) => {
                ui.strong("New callsite:");
                ui.label(callsite.location.to_string())
                    .on_hover_ui(|ui| self.span_tree.callsite_ui_by_id(ui, &callsite.id));
            }
            rr_data::MessageEnum::NewSpan(span) => {
                ui.strong("New span:");
                ui.label(self.span_tree.span_name(&span.id))
                    .on_hover_ui(|ui| {
                        self.span_tree.span_summary_ui_by_id(ui, &span.id);
                        ui.separator();
                        ui.heading("Callsite:");
                        self.span_tree.callsite_ui_by_id(ui, &span.callsite_id);
                    });
            }
            rr_data::MessageEnum::EnterSpan(span_id) => {
                ui.strong("Enter span:");
                ui.label(self.span_tree.span_name(span_id))
                    .on_hover_ui(|ui| {
                        self.span_tree.span_summary_ui_by_id(ui, span_id);
                    });
            }
            rr_data::MessageEnum::ExitSpan(span_id) => {
                ui.strong("Exit span:");
                ui.label(self.span_tree.span_name(span_id))
                    .on_hover_ui(|ui| {
                        self.span_tree.span_summary_ui_by_id(ui, span_id);
                    });
            }
            rr_data::MessageEnum::DestroySpan(span_id) => {
                ui.strong("Destroy span:");
                ui.label(self.span_tree.span_name(span_id))
                    .on_hover_ui(|ui| {
                        self.span_tree.span_summary_ui_by_id(ui, span_id);
                    });
            }
            rr_data::MessageEnum::SpanFollowsFrom { span, follows } => {
                ui.strong("Follows:");
                ui.label(format!(
                    "{} ➡ {}",
                    self.span_tree.span_name(follows),
                    self.span_tree.span_name(span),
                ))
                .on_hover_ui(|ui| {
                    self.span_tree.span_summary_ui_by_id(ui, span);
                    ui.separator();
                    ui.label("Follows:");
                    self.span_tree.span_summary_ui_by_id(ui, follows);
                });
            }
            rr_data::MessageEnum::DataEvent(data_event) => {
                ui.strong("Event:");
                self.span_tree.data_event_ui(ui, data_event);
            }
        }
    }
}
