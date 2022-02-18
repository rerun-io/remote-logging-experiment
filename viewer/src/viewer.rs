use eframe::egui;
use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};
use rr_data::TopicMeta;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum View {
    Events,
    Log,
    SpanTree,
    Flamegraph,
}

pub struct Viewer {
    ws_sender: WsSender,
    ws_receiver: WsReceiver,
    topics: Vec<TopicMeta>,
    view: View,
    /// What we are viewing
    topic_viewer: Option<TopicViewer>,
    full_event_log: crate::event_log::EventLog,
}

impl Viewer {
    pub fn new(ws_sender: WsSender, ws_receiver: WsReceiver) -> Self {
        Self {
            ws_sender,
            ws_receiver,
            topics: Default::default(),
            view: View::Flamegraph,
            topic_viewer: None,
            full_event_log: Default::default(),
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context) {
        while let Some(event) = self.ws_receiver.try_recv() {
            if let WsEvent::Opened = &event {
                tracing::info!("Web-socket connection opened.");
                self.ws_sender
                    .send(WsMessage::Binary(rr_data::PubSubMsg::ListTopics.encode()));
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
                                if let Some(topic_viewer) = &mut self.topic_viewer {
                                    topic_viewer.on_message(&rr_msg);
                                }
                                self.full_event_log.on_message(rr_msg);
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
                            tracing::debug!("Received {} topic(s)", all_topics.len());
                            self.topics = all_topics;
                            if self.topic_viewer.is_none() {
                                if let Some(latest_topic) = self.topics.last().cloned() {
                                    self.subscribe_to(latest_topic);
                                }
                            }
                        }
                    }
                    continue;
                }
            }
            self.full_event_log.on_text(format!("Recevied {:?}", event));
        }

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

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.selectable_value(&mut self.view, View::Events, "Events");
                    ui.selectable_value(&mut self.view, View::Log, "Log");
                    ui.selectable_value(&mut self.view, View::SpanTree, "Span tree");
                    ui.selectable_value(&mut self.view, View::Flamegraph, "Flame graph");
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.view {
            View::Events => {
                self.full_event_log.ui(ui);
            }
            View::Log => {
                if let Some(topic_viewer) = &mut self.topic_viewer {
                    topic_viewer.data_event_log.ui(ui, &topic_viewer.span_tree);
                }
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
            let topic_summary = format!("{} - {}", topic_meta.created.format(), topic_meta.name);
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
        self.full_event_log
            .on_text(format!("Subscribing to new topic: {:?}", topic_meta));
        self.ws_sender.send(WsMessage::Binary(
            rr_data::PubSubMsg::SubscribeTo(topic_meta.id).encode(),
        ));
        self.topic_viewer = Some(TopicViewer::new(topic_meta));
    }
}

pub struct TopicViewer {
    topic_meta: TopicMeta,
    span_tree: crate::span_tree::SpanTree,
    flame_graph: crate::flamegraph::FlameGraph,
    data_event_log: crate::data_event_log::DataEventLog,
}

impl TopicViewer {
    pub fn new(topic_meta: TopicMeta) -> Self {
        Self {
            topic_meta,
            span_tree: Default::default(),
            flame_graph: Default::default(),
            data_event_log: Default::default(),
        }
    }

    pub fn on_message(&mut self, rr_msg: &rr_data::Message) {
        self.data_event_log.on_message(rr_msg);
        self.span_tree.on_mesage(rr_msg);
    }
}
