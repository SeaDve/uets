use std::env;

pub fn bypass_wormhole() -> bool {
    env::var("BYPASS_WORMHOLE").is_ok_and(|s| s == "1")
}
