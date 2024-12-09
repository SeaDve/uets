use std::collections::HashMap;

use adw::{prelude::*, subclass::prelude::*};
use anyhow::Result;
use gtk::glib::{self, clone};
use serde::{Deserialize, Serialize};

use crate::{
    ai_chat_message::{AiChatMessage, AiChatMessageTy},
    ai_chat_message_list::AiChatMessageList,
    ui::ai_chat_message_row::AiChatMessageRow,
};

const API_KEY: &str = "AIzaSyD6aJhtX0rjAHkvEkpzJGCobtsy5AL1_aY";
const MODEL_NAME: &str = "gemini-1.5-flash-latest";

mod imp {
    use std::cell::OnceCell;

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/ai_chat_dialog.ui")]
    pub struct AiChatDialog {
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) empty_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub(super) main_page: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub(super) message_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) suggestion_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) suggestion_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) message_entry: TemplateChild<gtk::Entry>,

        pub(super) message_list: AiChatMessageList,

        pub(super) system_instruction: OnceCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AiChatDialog {
        const NAME: &'static str = "UetsAiChatDialog";
        type Type = super::AiChatDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("ai-chat-dialog.reset", None, |obj, _, _| {
                let imp = obj.imp();

                imp.message_list.clear();
            });
            klass.install_action_async(
                "ai-chat-dialog.send-message",
                None,
                |obj, _, _| async move {
                    let imp = obj.imp();

                    let message = imp.message_entry.text();
                    imp.message_entry.set_text("");

                    obj.handle_send_message(&message);
                },
            );
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AiChatDialog {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            self.message_list_box
                .bind_model(Some(&self.message_list), |o| {
                    let message = o.downcast_ref::<AiChatMessage>().unwrap();
                    let row = AiChatMessageRow::new(message);
                    row.upcast()
                });

            self.suggestion_list_box.connect_row_activated(clone!(
                #[weak]
                obj,
                move |_, row| {
                    let imp = obj.imp();

                    let label = row.child().unwrap().downcast::<gtk::Label>().unwrap();
                    obj.handle_send_message(&label.text());

                    imp.suggestion_button.popdown();
                }
            ));

            self.message_entry.connect_changed(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_send_message_action();
                }
            ));
            self.message_entry.connect_activate(clone!(
                #[weak]
                obj,
                move |_| {
                    WidgetExt::activate_action(&obj, "ai-chat-dialog.send-message", None).unwrap();
                }
            ));

            self.message_list.connect_items_changed(clone!(
                #[weak]
                obj,
                move |_, _, _, _| {
                    obj.update_stack();
                }
            ));

            obj.update_stack();
            obj.update_send_message_action();
        }

        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for AiChatDialog {}
    impl AdwDialogImpl for AiChatDialog {}
}

glib::wrapper! {
    pub struct AiChatDialog(ObjectSubclass<imp::AiChatDialog>)
        @extends gtk::Widget, adw::Dialog;
}

impl AiChatDialog {
    pub fn new(
        system_instruction: Option<impl Into<String>>,
        suggestions: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.system_instruction
            .set(system_instruction.map(|s| s.into()))
            .unwrap();

        for suggestion in suggestions {
            let button = gtk::Label::builder()
                .label(suggestion.into())
                .xalign(0.0)
                .build();
            imp.suggestion_list_box.append(&button);
        }

        this
    }

    fn handle_send_message(&self, text: &str) {
        let text = text.to_string();
        glib::spawn_future_local(clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                if let Err(err) = obj.handle_send_message_inner(&text).await {
                    tracing::error!("Failed to handle message: {:?}", err);
                }
            }
        ));
    }

    async fn handle_send_message_inner(&self, text: &str) -> Result<()> {
        let imp = self.imp();

        let user_message = AiChatMessage::new(AiChatMessageTy::User);
        user_message.set_loaded(text);
        imp.message_list.push(user_message);

        self.update_send_message_action();

        let payload =
            GenerateContentRequest {
                contents: imp
                    .message_list
                    .iter()
                    .filter_map(|message| {
                        let text = message.text()?;
                        Some(Content {
                            role: match message.ty() {
                                AiChatMessageTy::User => "user".to_string(),
                                AiChatMessageTy::Ai => "model".to_string(),
                            },
                            parts: vec![Part::Text(text)],
                        })
                    })
                    .collect(),
                generation_config: Some(GenerationConfig {
                    max_output_tokens: Some(2048),
                    temperature: Some(0.4),
                    top_p: Some(1.0),
                    top_k: Some(32),
                    ..Default::default()
                }),
                system_instruction: imp.system_instruction.get().unwrap().clone().map(
                    |instruction| Content {
                        role: "user".to_string(),
                        parts: vec![Part::Text(instruction)],
                    },
                ),
                tools: None,
            };

        let ai_message = AiChatMessage::new(AiChatMessageTy::Ai);
        imp.message_list.push(ai_message.clone());
        ai_message.set_loading();

        self.update_send_message_action();

        let endpoint_url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{MODEL_NAME}:generateContent?key={API_KEY}"
        );
        let response = async move {
            let bytes = surf::post(endpoint_url)
                .body_json(&payload)
                .map_err(|err| err.into_inner())?
                .await
                .map_err(|err| err.into_inner())?
                .body_bytes()
                .await
                .map_err(|err| err.into_inner())?;

            let value = serde_json::from_slice::<serde_json::Value>(&bytes)?;
            let response = serde_json::from_value::<GenerateContentResponse>(value)?;

            anyhow::Ok(response)
        };

        match response.await {
            Ok(response) => {
                if let Some(candidate) = response.candidates.first() {
                    tracing::debug!(
                        "Received {} parts from candidate",
                        candidate.content.parts.len()
                    );

                    let text_markup = candidate
                        .content
                        .parts
                        .iter()
                        .flat_map(|part| {
                            if let Part::Text(text) = part {
                                let text_html = markdown::to_html(text);
                                let text_markup = html2pango::markup_html(&text_html).unwrap();
                                Some(text_markup)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>();

                    ai_message.set_loaded_markup(text_markup.join("\n"));
                } else {
                    let text = "I'm sorry, I don't know how to respond to that.";
                    ai_message.set_loaded(text.to_string());
                }

                tracing::debug!("Received {} candidates", response.candidates.len());
            }
            Err(err) => {
                ai_message.set_loaded(format!("Error: {:?}", err));
            }
        }

        self.update_send_message_action();

        Ok(())
    }

    fn update_stack(&self) {
        let imp = self.imp();

        if imp.message_list.n_items() == 0 {
            imp.stack.set_visible_child(&*imp.empty_page);
        } else {
            imp.stack.set_visible_child(&*imp.main_page);
        }
    }

    fn update_send_message_action(&self) {
        let imp = self.imp();

        let is_enabled = !imp.message_entry.text().is_empty()
            && imp
                .message_list
                .last()
                .map_or(true, |message| message.is_loaded());
        self.action_set_enabled("ai-chat-dialog.send-message", is_enabled);
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FunctionDeclaration {
    name: String,
    description: String,
    parameters: FunctionParameters,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FunctionParameters {
    r#type: String,
    properties: HashMap<String, FunctionParametersProperty>,
    required: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FunctionParametersProperty {
    r#type: String,
    description: String,
}

#[derive(Serialize, Deserialize)]
struct Tools {
    function_declarations: Option<Vec<FunctionDeclaration>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum Part {
    Text(String),
    InlineData {
        mime_type: String,
        data: String,
    },
    FileData {
        mime_type: String,
        file_uri: String,
    },
    FunctionCall {
        name: String,
        args: HashMap<String, String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct Content {
    role: String,
    parts: Vec<Part>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    max_output_tokens: Option<i32>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    top_k: Option<i32>,
    stop_sequences: Option<Vec<String>>,
    candidate_count: Option<u32>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GenerateContentRequest {
    contents: Vec<Content>,
    generation_config: Option<GenerationConfig>,
    system_instruction: Option<Content>,
    tools: Option<Vec<Tools>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageMetadata {
    candidates_token_count: Option<i32>,
    prompt_token_count: i32,
    total_token_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GenerateContentResponse {
    candidates: Vec<Candidate>,
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Candidate {
    content: Content,
    citation_metadata: Option<CitationMetadata>,
    finish_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Citation {
    start_index: i32,
    end_index: i32,
    uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CitationMetadata {
    citations: Vec<Citation>,
}
