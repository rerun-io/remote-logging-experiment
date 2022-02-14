use eframe::{egui, epi};
use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};
use rr_data::TopicMeta;

use crate::misc::format_time;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum View {
    EventLog,
    SpanTree,
    Flamegraph,
}

pub struct Viewer {
    ws_sender: WsSender,
    ws_receiver: WsReceiver,
    topics: Vec<TopicMeta>,
    span_tree: crate::span_tree::SpanTree,
    view: View,
    /// What we are viewing
    topic_viewer: Option<TopicViewer>,
    event_log: crate::event_log::EventLog,
}

impl Viewer {
    pub fn new(mut ws_sender: WsSender, ws_receiver: WsReceiver) -> Self {
        ws_sender.send(WsMessage::Binary(rr_data::PubSubMsg::ListTopics.encode()));

        Self {
            ws_sender,
            ws_receiver,
            topics: Default::default(),
            span_tree: Default::default(),
            view: View::Flamegraph,
            topic_viewer: None,
            event_log: Default::default(),
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        while let Some(event) = self.ws_receiver.try_recv() {
            if let WsEvent::Opened = &event {
                tracing::info!("Web-socket connection opened.");
            }
            if let WsEvent::Message(WsMessage::Binary(payload)) = &event {
                if let Ok(pub_sub_msg) = rr_data::PubSubMsg::decode(payload) {
                    match pub_sub_msg {
                        rr_data::PubSubMsg::NewTopic(topic_meta) => {
                            if self.topic_viewer.is_none() {
                                self.subscribe_to(topic_meta);
                            }

                            // Refresh list
                            self.ws_sender
                                .send(WsMessage::Binary(rr_data::PubSubMsg::ListTopics.encode()));

                            continue;
                        }
                        rr_data::PubSubMsg::TopicMsg(_topic_id, payload) => {
                            if let Ok(rr_msg) = rr_data::Message::decode(&payload) {
                                self.span_tree.on_mesage(&rr_msg);
                                if let Some(topic_viewer) = &mut self.topic_viewer {
                                    topic_viewer.on_message(&rr_msg);
                                }
                                self.event_log.on_message(rr_msg);
                                continue;
                            }
                        }
                        rr_data::PubSubMsg::SubscribeTo(_) => {
                            // weird
                        }
                        rr_data::PubSubMsg::ListTopics => {
                            tracing::debug!("Server sent ListTopics message. Weird");
                        }
                        rr_data::PubSubMsg::AllTopics(all_topics) => {
                            self.topics = all_topics;
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

                egui::menu::bar(ui, |ui| {
                    ui.label("View:");
                    ui.selectable_value(&mut self.view, View::EventLog, "Event log");
                    ui.selectable_value(&mut self.view, View::SpanTree, "Span tree");
                    ui.selectable_value(&mut self.view, View::Flamegraph, "Flame graph");
                });
            });
        });

        egui::SidePanel::left("left_bar")
            .resizable(false)
            .show(ctx, |ui| {
                ui.style_mut().wrap = Some(false);
                ui.heading("Available topics:");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let clicked = self.show_topic_list(ui);
                    if let Some(topic_meta) = clicked {
                        self.subscribe_to(topic_meta);
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| match self.view {
            View::EventLog => {
                self.event_log.ui(ui, &self.span_tree);
            }
            View::SpanTree => {
                if let Some(topic_viewer) = &mut self.topic_viewer {
                    topic_viewer.span_tree.tree_ui(ui);
                }
            }
            View::Flamegraph => {
                if let Some(topic_viewer) = &mut self.topic_viewer {
                    topic_viewer.flame_graph.ui(ui, &topic_viewer.span_tree);
                }
            }
        });
    }

    fn show_topic_list(&self, ui: &mut egui::Ui) -> Option<TopicMeta> {
        let mut clicked = None;
        for topic_meta in &self.topics {
            let topic_summary =
                format!("{} - {}", format_time(&topic_meta.created), topic_meta.name);
            let is_selected = self
                .topic_viewer
                .as_ref()
                .map_or(false, |viewer| viewer.topic_meta.id == topic_meta.id);
            if ui.selectable_label(is_selected, topic_summary).clicked() {
                clicked = Some(topic_meta.clone());
            }
        }
        clicked
    }

    fn subscribe_to(&mut self, topic_meta: TopicMeta) {
        tracing::info!("Subscribing to new topic: {:?}", topic_meta);
        self.event_log
            .on_text(format!("Subscribing to new topic: {:?}", topic_meta));
        self.ws_sender.send(WsMessage::Binary(
            rr_data::PubSubMsg::SubscribeTo(topic_meta.id).encode(),
        ));
        self.topic_viewer = Some(TopicViewer::new(topic_meta.clone()));
    }
}

pub struct TopicViewer {
    topic_meta: TopicMeta,
    span_tree: crate::span_tree::SpanTree,
    flame_graph: crate::flamegraph::FlameGraph,
}

impl TopicViewer {
    pub fn new(topic_meta: TopicMeta) -> Self {
        Self {
            topic_meta,
            span_tree: Default::default(),
            flame_graph: Default::default(),
        }
    }

    pub fn on_message(&mut self, rr_msg: &rr_data::Message) {
        self.span_tree.on_mesage(rr_msg);
    }
}
