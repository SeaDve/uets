use std::collections::HashMap;

use adw::{prelude::*, subclass::prelude::*};
use anyhow::Result;
use gtk::glib::{self, clone, closure_local};
use serde::{Deserialize, Serialize};

use crate::{
    ai_chat_message::{AiChatMessage, AiChatMessageTy},
    ai_chat_message_list::AiChatMessageList,
    ui::ai_chat_message_row::AiChatMessageRow,
};

const API_KEY: &str = "AIzaSyD6aJhtX0rjAHkvEkpzJGCobtsy5AL1_aY";
const MODEL_NAME: &str = "gemini-1.5-flash-latest";

mod imp {
    use std::{
        cell::{Cell, OnceCell, RefCell},
        sync::OnceLock,
    };

    use glib::subclass::Signal;

    use super::*;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::AiChatDialog )]
    #[template(resource = "/io/github/seadve/Uets/ui/ai_chat_dialog.ui")]
    pub struct AiChatDialog {
        #[property(get, set = Self::set_message_list, construct_only)]
        pub(super) message_list: OnceCell<AiChatMessageList>,

        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) empty_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub(super) main_page: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub(super) message_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) scroll_to_bottom_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub(super) suggestion_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) suggestion_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) message_entry: TemplateChild<gtk::Entry>,

        pub(super) message_list_items_changed_id: RefCell<Option<glib::SignalHandlerId>>,

        pub(super) system_instruction: OnceCell<Option<String>>,

        pub(super) is_sticky: Cell<bool>,
        pub(super) is_auto_scrolling: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AiChatDialog {
        const NAME: &'static str = "UetsAiChatDialog";
        type Type = super::AiChatDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("ai-chat-dialog.reset", None, |obj, _, _| {
                obj.message_list().clear();
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
            klass.install_action("ai-chat-dialog.scroll-to-bottom", None, |obj, _, _| {
                obj.scroll_to_bottom();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for AiChatDialog {
        fn constructed(&self) {
            self.parent_constructed();

            self.is_sticky.set(true);

            let obj = self.obj();

            let vadj = self.main_page.vadjustment();
            vadj.connect_value_changed(clone!(
                #[weak]
                obj,
                move |_| {
                    let imp = obj.imp();

                    let is_at_bottom = obj.is_at_bottom();
                    if imp.is_auto_scrolling.get() {
                        if is_at_bottom {
                            imp.is_auto_scrolling.set(false);
                            imp.is_sticky.set(true);
                        } else {
                            obj.scroll_to_bottom();
                        }
                    } else {
                        imp.is_sticky.set(is_at_bottom);
                    }

                    obj.update_scroll_to_bottom_revealer_reveal_child();
                }
            ));
            vadj.connect_upper_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    let imp = obj.imp();

                    if imp.is_sticky.get() {
                        obj.scroll_to_bottom();
                    }

                    obj.update_scroll_to_bottom_revealer_reveal_child();
                }
            ));
            vadj.connect_page_size_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    let imp = obj.imp();

                    if imp.is_sticky.get() {
                        obj.scroll_to_bottom();
                    }

                    obj.update_scroll_to_bottom_revealer_reveal_child();
                }
            ));

            self.scroll_to_bottom_revealer
                .connect_child_revealed_notify(clone!(
                    #[weak]
                    obj,
                    move |_| {
                        obj.update_scroll_to_bottom_revealer_can_target();
                    }
                ));

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

            obj.update_stack();
            obj.update_send_message_action();
            obj.update_scroll_to_bottom_revealer_reveal_child();
            obj.update_scroll_to_bottom_revealer_can_target();
        }

        fn dispose(&self) {
            self.dispose_template();

            let obj = self.obj();

            if let Some(handler_id) = self.message_list_items_changed_id.take() {
                obj.message_list().disconnect(handler_id);
            }
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

            SIGNALS.get_or_init(|| {
                vec![Signal::builder("activate-link")
                    .param_types([String::static_type()])
                    .return_type::<bool>()
                    .build()]
            })
        }
    }

    impl WidgetImpl for AiChatDialog {}
    impl AdwDialogImpl for AiChatDialog {}

    impl AiChatDialog {
        fn set_message_list(&self, message_list: AiChatMessageList) {
            let obj = self.obj();

            self.message_list_box.bind_model(
                Some(&message_list),
                clone!(
                    #[weak]
                    obj,
                    #[upgrade_or_panic]
                    move |o| {
                        let message = o.downcast_ref::<AiChatMessage>().unwrap();

                        let row = AiChatMessageRow::new(message);
                        row.connect_activate_link(clone!(
                            #[weak]
                            obj,
                            #[upgrade_or_panic]
                            move |_, uri| obj.emit_by_name::<bool>("activate-link", &[&uri]).into()
                        ));

                        row.upcast()
                    }
                ),
            );

            let handler_id = message_list.connect_items_changed(clone!(
                #[weak]
                obj,
                move |_, _, _, _| {
                    obj.update_stack();
                }
            ));
            self.message_list_items_changed_id.replace(Some(handler_id));

            self.message_list.set(message_list).unwrap();
        }
    }
}

glib::wrapper! {
    pub struct AiChatDialog(ObjectSubclass<imp::AiChatDialog>)
        @extends gtk::Widget, adw::Dialog;
}

impl AiChatDialog {
    pub fn new(
        message_list: &AiChatMessageList,
        system_instruction: Option<impl Into<String>>,
        suggestions: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        let this = glib::Object::builder::<Self>()
            .property("message-list", message_list)
            .build();

        let imp = this.imp();
        imp.system_instruction
            .set(system_instruction.map(|s| s.into()))
            .unwrap();

        for suggestion in suggestions {
            let button = gtk::Label::builder()
                .label(suggestion.into())
                .wrap(true)
                .xalign(0.0)
                .build();
            imp.suggestion_list_box.append(&button);
        }

        this
    }

    pub fn connect_activate_link<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &str) -> glib::Propagation + 'static,
    {
        self.connect_closure(
            "activate-link",
            false,
            closure_local!(|obj: &Self, uri: &str| f(obj, uri)),
        )
    }

    pub fn send_message(&self, text: &str) {
        self.handle_send_message(text);
    }

    fn scroll_to_bottom(&self) {
        let imp = self.imp();

        imp.is_auto_scrolling.set(true);
        imp.main_page.emit_scroll_child(gtk::ScrollType::End, false);

        self.update_scroll_to_bottom_revealer_reveal_child();
    }

    fn is_at_bottom(&self) -> bool {
        let imp = self.imp();

        let vadj = imp.main_page.vadjustment();
        vadj.value() + vadj.page_size() == vadj.upper()
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
        self.message_list().push(user_message);

        self.update_send_message_action();

        let payload =
            GenerateContentRequest {
                contents: self
                    .message_list()
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
                    temperature: Some(0.2),
                    top_p: Some(0.5),
                    top_k: Some(8),
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
        self.message_list().push(ai_message.clone());
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
                    tracing::trace!(
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

                    ai_message.set_loaded_markup(text_markup.join("\n").trim());
                } else {
                    let text = "I'm sorry, I don't know how to respond to that.";
                    ai_message.set_loaded(text.to_string());
                }

                tracing::trace!("Received {} candidates", response.candidates.len());
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

        if self.message_list().n_items() == 0 {
            imp.stack.set_visible_child(&*imp.empty_page);
        } else {
            imp.stack.set_visible_child(&*imp.main_page);
        }
    }

    fn update_send_message_action(&self) {
        let imp = self.imp();

        let is_enabled = !imp.message_entry.text().is_empty()
            && self
                .message_list()
                .last()
                .map_or(true, |message| message.is_loaded());
        self.action_set_enabled("ai-chat-dialog.send-message", is_enabled);
    }

    fn update_scroll_to_bottom_revealer_reveal_child(&self) {
        let imp = self.imp();

        imp.scroll_to_bottom_revealer
            .set_reveal_child(!self.is_at_bottom() && !imp.is_auto_scrolling.get());
    }

    fn update_scroll_to_bottom_revealer_can_target(&self) {
        let imp = self.imp();

        imp.scroll_to_bottom_revealer
            .set_can_target(imp.scroll_to_bottom_revealer.is_child_revealed());
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
