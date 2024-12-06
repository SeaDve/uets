use anyhow::{bail, ensure, Context, Result};
use gtk::{glib, subclass::prelude::*};

const PORT: u16 = 8888;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayState {
    #[default]
    Low,
    High,
}

mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default)]
    pub struct Relay {
        pub(super) ip_addr: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Relay {
        const NAME: &'static str = "UetsRelay";
        type Type = super::Relay;
    }

    impl ObjectImpl for Relay {}
}

glib::wrapper! {
    pub struct Relay(ObjectSubclass<imp::Relay>);
}

impl Relay {
    pub fn new(ip_addr: String) -> Self {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.ip_addr.replace(ip_addr);

        this
    }

    pub fn set_ip_addr(&self, ip_addr: String) {
        let imp = self.imp();

        imp.ip_addr.replace(ip_addr);
    }

    pub async fn state(&self) -> Result<RelayState> {
        let state = self
            .http_get("state")
            .await?
            .body_string()
            .await
            .map_err(|err| err.into_inner())?;

        match state.as_str() {
            "0" => Ok(RelayState::Low),
            "1" => Ok(RelayState::High),
            _ => bail!("Invalid state `{}`", state),
        }
    }

    pub async fn set_state(&self, state: RelayState) -> Result<()> {
        let path = match state {
            RelayState::Low => "low",
            RelayState::High => "high",
        };

        self.http_get(path).await?;
        tracing::debug!("Relay state set to {:?}", state);

        Ok(())
    }

    async fn http_get(&self, path: &str) -> Result<surf::Response> {
        let imp = self.imp();

        let uri = format!("http://{}:{PORT}/{path}", imp.ip_addr.borrow());
        let response = surf::RequestBuilder::new(
            surf::http::Method::Get,
            uri.parse()
                .with_context(|| format!("Failed to parse URI: {}", uri))?,
        )
        .send()
        .await
        .map_err(|err| err.into_inner())?;

        ensure!(
            response.status().is_success(),
            "Failed to send GET request at {}",
            uri
        );

        Ok(response)
    }
}
