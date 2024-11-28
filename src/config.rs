use std::env;

pub fn bypass_wormhole() -> bool {
    env::var("BYPASS_WORMHOLE").is_ok_and(|s| s == "1")
}

pub fn disable_camera_detector() -> bool {
    env::var("DISABLE_CAMERA_DETECTOR").is_ok_and(|s| s == "1")
}
