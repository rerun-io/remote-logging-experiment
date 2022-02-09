use eframe::egui;
use rr_data::{SpanId, Time};
use std::collections::{HashMap, HashSet};

use crate::misc::format_location;

#[derive(Default)]
pub struct FlameGraph {
    callsites: HashMap<rr_data::CallsiteId, rr_data::Callsite>,
    nodes: HashMap<SpanId, SpanNode>,
    roots: HashSet<SpanId>,
    orphan_events: Vec<rr_data::DataEvent>,
}

/// A span is created, and then is opened over many non-overlapping intervals.
#[derive(Debug)]
struct SpanNode {
    span: rr_data::Span,
    created: Time,
    intervals: Vec<TimeInterval>,
    children: HashSet<SpanId>,
    events: Vec<rr_data::DataEvent>,
}

#[derive(Debug, Default)]
struct TimeInterval {
    entered: Option<Time>,
    exited: Option<Time>,
}

impl std::fmt::Display for TimeInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn format_optional_time(time: Option<Time>) -> String {
            match time {
                Some(time) => crate::misc::format_time(&time),
                None => "?".to_owned(),
            }
        }

        write!(
            f,
            "[{} - {}]",
            format_optional_time(self.entered),
            format_optional_time(self.exited)
        )
    }
}

impl FlameGraph {
    pub fn on_mesage(&mut self, message: &rr_data::Message) {
        let rr_data::Message { log_time, msg_enum } = message;
        match &msg_enum {
            rr_data::MessageEnum::NewCallsite(callsite) => {
                self.callsites.insert(callsite.id, callsite.clone());
            }
            rr_data::MessageEnum::NewSpan(span) => {
                let prev = self.nodes.insert(
                    span.id,
                    SpanNode {
                        span: span.clone(),
                        created: *log_time,
                        children: Default::default(),
                        intervals: Default::default(),
                        events: Default::default(),
                    },
                );
                if prev.is_some() {
                    tracing::warn!("Reused span id");
                }

                if let Some(parent_span_id) = &span.parent_span_id {
                    if let Some(parent_node) = self.nodes.get_mut(parent_span_id) {
                        parent_node.children.insert(span.id);
                    } else {
                        tracing::warn!("Unknown parent span");
                    }
                } else {
                    self.roots.insert(span.id);
                }
            }

            rr_data::MessageEnum::EnterSpan(span_id) => {
                if let Some(node) = self.nodes.get_mut(span_id) {
                    node.intervals.push(TimeInterval {
                        entered: Some(*log_time),
                        exited: None,
                    });
                } else {
                    tracing::warn!("Opened unknown span");
                }
            }
            rr_data::MessageEnum::ExitSpan(span_id) => {
                if let Some(node) = self.nodes.get_mut(span_id) {
                    if let Some(interval) = node.intervals.last_mut() {
                        if interval.exited.is_none() {
                            interval.exited = Some(*log_time);
                        } else {
                            tracing::warn!("Exited span that was never opened");
                            node.intervals.push(TimeInterval {
                                entered: None,
                                exited: Some(*log_time),
                            });
                        }
                    } else {
                        tracing::warn!("Exited span that was never opened");
                        node.intervals.push(TimeInterval {
                            entered: None,
                            exited: Some(*log_time),
                        });
                    }
                } else {
                    tracing::warn!("Exited unknown span");
                }
            }
            rr_data::MessageEnum::DataEvent(event) => {
                if let Some(span_id) = &event.parent_span_id {
                    if let Some(node) = self.nodes.get_mut(span_id) {
                        node.events.push(event.clone());
                    } else {
                        tracing::warn!("Event with unknown span");
                    }
                } else {
                    self.orphan_events.push(event.clone());
                }
            }
        }
    }

    pub fn tree_ui(&self, ui: &mut egui::Ui) {
        for &span_id in &self.roots {
            self.node_ui(ui, 0, span_id);
        }
    }

    fn node_ui(&self, ui: &mut egui::Ui, depth: usize, span_id: SpanId) {
        if let Some(node) = self.nodes.get(&span_id) {
            if let Some(callsite) = self.callsites.get(&node.span.callsite_id) {
                egui::CollapsingHeader::new(callsite.name.as_str())
                    .id_source(span_id)
                    .default_open(depth < 4)
                    .show(ui, |ui| {
                        self.node_innards(ui, depth, node, callsite);
                    });
            } else {
                ui.colored_label(egui::Color32::RED, "Missing callsite");
            }
        } else {
            ui.colored_label(egui::Color32::RED, "Missing span");
        }
    }

    fn node_innards(
        &self,
        ui: &mut egui::Ui,
        depth: usize,
        node: &SpanNode,
        callsite: &rr_data::Callsite,
    ) {
        ui.label(format!("Level: {}", callsite.level));
        ui.label(format!("Location: {}", format_location(&callsite.location)));

        let SpanNode {
            span: _,
            created,
            children,
            intervals,
            events,
        } = node;

        use itertools::Itertools as _;

        ui.label(format!("Created: {}", crate::misc::format_time(created)));
        ui.label(format!("Intervals: {}", intervals.iter().format(", ")));

        if events.is_empty() {
            ui.label("Events: [NONE]");
        } else {
            ui.label("Events:");
            ui.indent("events", |ui| {
                for event in events {
                    self.ui_data_event(ui, event);
                }
            });
        }

        for &child in children {
            self.node_ui(ui, depth + 1, child);
        }
    }

    fn ui_data_event(&self, ui: &mut egui::Ui, data_event: &rr_data::DataEvent) {
        let rr_data::DataEvent {
            callsite_id,
            parent_span_id: _,
            fields,
        } = data_event;

        let response = ui.horizontal(|ui| {
            for (key, value) in fields {
                ui.label(egui::RichText::new(format!("{}: ", key)).weak());
                ui.label(value.to_string());
            }
        });

        response.response.on_hover_ui(|ui| {
            ui.heading("Callsite:");
            self.ui_callsite_id(ui, callsite_id);
        });
    }

    fn ui_callsite_id(&self, ui: &mut egui::Ui, callsite_id: &rr_data::CallsiteId) {
        if let Some(callsite) = self.callsites.get(callsite_id) {
            crate::misc::ui_callsite(ui, callsite);
        } else {
            ui.label(format!("Unknown callsite: {}", callsite_id)); // error
        }
    }
}
