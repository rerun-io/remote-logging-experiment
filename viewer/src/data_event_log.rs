use eframe::egui;

use crate::span_tree::SpanTree;

// ----------------------------------------------------------------------------

/// View every log event
#[derive(Default)]
pub struct DataEventLog {
    events: Vec<(rr_data::Time, rr_data::DataEvent)>,
}

impl DataEventLog {
    pub fn on_message(&mut self, msg: &rr_data::Message) {
        if let rr_data::MessageEnum::DataEvent(data_event) = &msg.msg_enum {
            self.events.push((msg.log_time, data_event.clone()));
        }
    }

    pub fn ui(&self, ui: &mut egui::Ui, span_tree: &SpanTree) {
        ui.label("Hover to view call sites");
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for (time, data_event) in &self.events {
                    self.ui_event(ui, span_tree, *time, data_event);
                }
            });
    }

    fn ui_event(
        &self,
        ui: &mut egui::Ui,
        span_tree: &SpanTree,
        log_time: rr_data::Time,
        data_event: &rr_data::DataEvent,
    ) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(log_time.format()).weak().monospace());
            span_tree.data_event_ui(ui, data_event);
        });
    }
}
