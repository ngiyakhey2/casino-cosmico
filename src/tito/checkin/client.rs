pub mod checkin_lists_handler;

use checkin_lists_handler::CheckinListsHandler;

const TITO_API_BASE: &str = "https://checkin.tito.io";

#[derive(thiserror::Error, Debug)]
pub enum ClientBuilderError {
    #[error("Could not construct client from builder")]
    BuilderError(#[from] reqwest::Error),
}

/// Builder for constructing
pub struct ClientBuilder {
    client: reqwest::Client,
    base_url: Option<String>,
}

impl ClientBuilder {
    pub fn new() -> Result<Self, ClientBuilderError> {
        Ok(Self {
            client: reqwest::ClientBuilder::new().build()?,
            base_url: None,
        })
    }

    pub fn base_url(&mut self, url: impl Into<String>) -> &ClientBuilder {
        self.base_url = Some(url.into());
        self
    }

    pub fn build(self) -> Client {
        Client {
            client: self.client,
            base_url: self.base_url.unwrap_or_else(|| TITO_API_BASE.to_string()),
        }
    }
}

#[derive(Clone)]
pub struct Client {
    client: reqwest::Client,
    base_url: String,
}

impl<'a> Client {
    pub fn check_ins(&'a self, checkin_lists_slug: &str) -> CheckinListsHandler<'a> {
        CheckinListsHandler::new(self, checkin_lists_slug)
    }
}
