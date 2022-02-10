use eframe::{egui, epi};
use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum View {
    EventLog,
    SpanTree,
    Flamegraph,
}

pub struct Viewer {
    ws_sender: WsSender,
    ws_receiver: WsReceiver,
    view: View,
    span_tree: crate::span_tree::SpanTree,
    event_log: crate::event_log::EventLog,
    flame_graph: crate::flamegraph::FlameGraph,
}

impl Viewer {
    pub fn new(ws_sender: WsSender, ws_receiver: WsReceiver) -> Self {
        Self {
            ws_sender,
            ws_receiver,
            view: View::Flamegraph,
            span_tree: Default::default(),
            event_log: Default::default(),
            flame_graph: Default::default(),
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        while let Some(event) = self.ws_receiver.try_recv() {
            if let WsEvent::Message(WsMessage::Binary(payload)) = &event {
                if let Ok(pub_sub_msg) = rr_data::PubSubMsg::decode(payload) {
                    match pub_sub_msg {
                        rr_data::PubSubMsg::NewTopic(topic_id, topic_meta) => {
                            self.event_log
                                .on_text(format!("Subscribing to new topic: {:?}", topic_meta));
                            self.ws_sender.send(WsMessage::Binary(
                                rr_data::PubSubMsg::SubscribeTo(topic_id).encode(),
                            ));
                            continue;
                        }
                        rr_data::PubSubMsg::TopicMsg(_topic_id, payload) => {
                            if let Ok(rr_msg) = rr_data::Message::decode(&payload) {
                                self.span_tree.on_mesage(&rr_msg);
                                self.event_log.on_message(rr_msg);
                                continue;
                            }
                        }
                        rr_data::PubSubMsg::SubscribeTo(_) => {
                            // weird
                        }
                    }
                    continue;
                }
            }
            self.event_log.on_text(format!("Recevied {:?}", event));
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                });
                ui.separator();
                ui.label("View:");
                ui.selectable_value(&mut self.view, View::EventLog, "Event log");
                ui.selectable_value(&mut self.view, View::SpanTree, "Span tree");
                ui.selectable_value(&mut self.view, View::Flamegraph, "Flame graph");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.view {
            View::EventLog => {
                self.event_log.ui(ui, &self.span_tree);
            }
            View::SpanTree => {
                self.span_tree.tree_ui(ui);
            }
            View::Flamegraph => {
                self.flame_graph.ui(ui, &self.span_tree);
            }
        });
    }
}
