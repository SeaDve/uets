use std::env;

pub fn bypass_wormhole() -> bool {
    env::var("BYPASS_WORMHOLE").is_ok_and(|s| s == "1")
}

/// This is a value from Google AI Studio: https://aistudio.google.com/apikey
pub fn ai_chat_api_key() -> String {
    env::var("AI_CHAT_API_KEY").unwrap_or_else(|_| "".to_string())
}
