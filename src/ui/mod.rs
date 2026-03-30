use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use eframe::egui;
use egui::{Color32, RichText, ScrollArea, TextEdit, Window};
use futures::stream::StreamExt;
use tokio::runtime::Runtime;

use crate::provider::{ChatRequest, LLMProvider, ModelInfo, ollama::OllamaProvider};
use crate::models::{Conversation, Message};
use crate::db::Database;
use crate::config::AppConfig;
use uuid::Uuid;

struct StreamState {
    conversation_id: Option<Uuid>,
    response: String,
    done: bool,
    error: Option<String>,
}

impl Default for StreamState {
    fn default() -> Self {
        Self {
            conversation_id: None,
            response: String::new(),
            done: false,
            error: None,
        }
    }
}

pub struct RustyLlamaApp {
    config: AppConfig,
    db: Database,
    provider: Arc<dyn LLMProvider>,
    runtime: Arc<Runtime>,
    conversations: Vec<Conversation>,
    current_conversation: Option<Uuid>,
    input_text: String,
    streaming_response: Option<String>,
    is_streaming: bool,
    stream_state: Arc<Mutex<StreamState>>,
    models: Arc<Mutex<Vec<ModelInfo>>>,
    show_model_window: bool,
}

impl RustyLlamaApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config = AppConfig::load();
        let db_path = config.data_dir.join("conversations.db");
        let db = Database::new(db_path).expect("Failed to open database");
        let provider = Arc::new(OllamaProvider::new(config.ollama.clone()));
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Failed to start Tokio runtime"),
        );

        let conversations = db.load_conversations().unwrap_or_default();
        let models = Arc::new(Mutex::new(Vec::new()));
        let stream_state = Arc::new(Mutex::new(StreamState::default()));

        let mut app = Self {
            config,
            db,
            provider: provider.clone(),
            runtime: runtime.clone(),
            conversations,
            current_conversation: None,
            input_text: String::new(),
            streaming_response: None,
            is_streaming: false,
            stream_state,
            models,
            show_model_window: false,
        };

        app.load_model_list();
        app
    }

    fn load_model_list(&mut self) {
        let provider = self.provider.clone();
        let models = self.models.clone();

        self.runtime.spawn(async move {
            if let Ok(list) = provider.list_models().await {
                if let Ok(mut locked) = models.lock() {
                    *locked = list;
                }
            }
        });
    }

    fn create_new_conversation(&mut self) {
        let conv = Conversation::new(
            self.config.default_model.clone(),
            self.config.default_system_prompt.clone(),
        );
        self.conversations.insert(0, conv.clone());
        self.current_conversation = Some(conv.id);
        self.save_conversation(&conv);
    }

    fn save_conversation(&self, conv: &Conversation) {
        if let Err(e) = self.db.save_conversation(conv) {
            eprintln!("Failed to save conversation: {}", e);
        }
    }

    fn start_chat(&mut self) {
        if self.input_text.trim().is_empty() || self.is_streaming {
            return;
        }

        let conversation_id = self.current_conversation.unwrap_or_else(|| {
            self.create_new_conversation();
            self.current_conversation.unwrap()
        });

        if let Some(conv) = self
            .conversations
            .iter_mut()
            .find(|c| c.id == conversation_id)
        {
            let user_msg = Message::new("user".to_string(), self.input_text.clone());
            conv.messages.push(user_msg);
            conv.updated_at = chrono::Utc::now();
            self.save_conversation(conv);

            let messages: Vec<_> = conv
                .messages
                .iter()
                .cloned()
                .map(|msg| msg.into())
                .collect();

            let request = ChatRequest {
                model: conv.model.clone(),
                messages,
                stream: true,
                options: HashMap::new(),
            };

            self.input_text.clear();
            self.is_streaming = true;
            self.streaming_response = Some(String::new());

            let provider = self.provider.clone();
            let stream_state = self.stream_state.clone();
            let convo_id = conv.id;

            self.runtime.spawn(async move {
                {
                    let mut state = stream_state.lock().unwrap();
                    state.conversation_id = Some(convo_id);
                    state.response.clear();
                    state.done = false;
                    state.error = None;
                }

                match provider.chat_stream(request).await {
                    Ok(mut response_stream) => {
                        let mut output = String::new();
                        while let Some(item) = response_stream.next().await {
                            match item {
                                Ok(chat_response) => {
                                    if let Some(msg) = chat_response.message {
                                        output.push_str(&msg.content);
                                    }
                                    if chat_response.done {
                                        break;
                                    }
                                }
                                Err(err) => {
                                    let mut state = stream_state.lock().unwrap();
                                    state.error = Some(err.to_string());
                                    state.done = true;
                                    return;
                                }
                            }

                            let mut state = stream_state.lock().unwrap();
                            state.response = output.clone();
                        }

                        let mut state = stream_state.lock().unwrap();
                        state.response = output;
                        state.done = true;
                    }
                    Err(err) => {
                        let mut state = stream_state.lock().unwrap();
                        state.error = Some(err.to_string());
                        state.done = true;
                    }
                }
            });
        }
    }

    fn check_stream_state(&mut self) {
        let mut state = self.stream_state.lock().unwrap();
        if self.is_streaming {
            self.streaming_response = Some(state.response.clone());
        }

        if self.is_streaming && state.done {
            if let Some(conversation_id) = state.conversation_id {
                if let Some(conv) = self
                    .conversations
                    .iter_mut()
                    .find(|c| c.id == conversation_id)
                {
                    let content = state
                        .error
                        .take()
                        .unwrap_or_else(|| state.response.clone());
                    conv.messages.push(Message::new("assistant".to_string(), content));
                    conv.updated_at = chrono::Utc::now();
                    self.save_conversation(conv);
                }
            }

            self.is_streaming = false;
            self.streaming_response = None;
            state.conversation_id = None;
            state.response.clear();
            state.done = false;
            state.error = None;
        }
    }
}

impl eframe::App for RustyLlamaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.config.theme {
            crate::config::Theme::Dark => ctx.set_visuals(egui::Visuals::dark()),
            crate::config::Theme::Light => ctx.set_visuals(egui::Visuals::light()),
            crate::config::Theme::System => {},
        }

        self.check_stream_state();

        if self.is_streaming {
            ctx.request_repaint();
        }

        egui::SidePanel::left("sidebar").show(ctx, |ui| {
            ui.heading("Conversations");
            if ui.button("New Chat").clicked() {
                self.create_new_conversation();
            }
            ui.separator();

            ScrollArea::vertical().show(ui, |ui| {
                for conv in &self.conversations {
                    let selected = Some(conv.id) == self.current_conversation;
                    let mut text = RichText::new(&conv.title);
                    if selected {
                        text = text.color(Color32::YELLOW);
                    }

                    if ui.selectable_label(selected, text).clicked() {
                        self.current_conversation = Some(conv.id);
                    }
                }
            });

            ui.separator();
            if ui.button("Select Model").clicked() {
                self.show_model_window = true;
            }
        });

        if self.show_model_window {
            Window::new("Select Model")
                .open(&mut self.show_model_window)
                .show(ctx, |ui| {
                    let models = self.models.lock().unwrap();
                    if models.is_empty() {
                        ui.label("Loading models...");
                    } else {
                        for model in models.iter() {
                            let selected = self
                                .current_conversation
                                .and_then(|id| {
                                    self.conversations
                                        .iter()
                                        .find(|c| c.id == id)
                                        .map(|c| c.model == model.name)
                                })
                                .unwrap_or(false);

                            if ui
                                .selectable_label(selected, &model.name)
                                .clicked()
                            {
                                if let Some(current_id) = self.current_conversation {
                                    if let Some(conv) = self
                                        .conversations
                                        .iter_mut()
                                        .find(|c| c.id == current_id)
                                    {
                                        conv.model = model.name.clone();
                                        conv.updated_at = chrono::Utc::now();
                                        self.save_conversation(conv);
                                    }
                                }
                            }
                        }
                    }
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(conv_id) = self.current_conversation {
                if let Some(conv) = self.conversations.iter().find(|c| c.id == conv_id) {
                    ui.heading(&conv.title);
                    ui.label(format!("Model: {}", conv.model));
                    ui.separator();

                    ScrollArea::vertical().show(ui, |ui| {
                        for msg in &conv.messages {
                            ui.group(|ui| {
                                let role = if msg.role == "assistant" {
                                    RichText::new(&msg.role).color(Color32::LIGHT_BLUE)
                                } else {
                                    RichText::new(&msg.role).strong()
                                };
                                ui.label(role);
                                ui.label(&msg.content);
                            });
                            ui.separator();
                        }

                        if self.is_streaming {
                            ui.group(|ui| {
                                ui.label(RichText::new("assistant").color(Color32::LIGHT_BLUE));
                                ui.label(self.streaming_response.as_ref().unwrap_or(&String::new()));
                            });
                        }
                    });

                    ui.separator();
                    ui.horizontal(|ui| {
                        let response = ui.add(TextEdit::multiline(&mut self.input_text).desired_rows(4));
                        if ui.button("Send").clicked()
                            || (response.lost_focus()
                                && ui.input(|i| {
                                    i.key_pressed(egui::Key::Enter) && !i.modifiers.shift
                                }))
                        {
                            self.start_chat();
                        }
                    });
                }
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Select a conversation or create a new one.");
                });
            }
        });
    }
}
