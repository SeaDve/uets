use wormhole::{
    rendezvous,
    transfer::{self, AppVersion},
    transit, AppConfig, AppID,
};

pub const DEFAULT_TRANSIT_ABILITIES: transit::Abilities = transit::Abilities::FORCE_DIRECT;

const APP_ID: &str = "lothar.com/wormhole/text-or-file-xfer";
const APP_RENDEZVOUS_URL: &str = rendezvous::DEFAULT_RENDEZVOUS_SERVER;
const TRANSIT_RELAY_URL: &str = transit::DEFAULT_RELAY_SERVER;

pub fn app_config() -> AppConfig<AppVersion> {
    AppConfig {
        id: AppID::new(APP_ID),
        rendezvous_url: APP_RENDEZVOUS_URL.into(),
        app_version: transfer::AppVersion::default(),
    }
}

pub fn relay_hints() -> Vec<transit::RelayHint> {
    vec![transit::RelayHint::from_urls(None, [TRANSIT_RELAY_URL.parse().unwrap()]).unwrap()]
}
