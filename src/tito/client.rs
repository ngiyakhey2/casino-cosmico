pub mod tickets_handler;

use reqwest::header;
use tickets_handler::TicketsHandler;

const TITO_API_BASE: &str = "https://api.tito.io/v3/";

#[derive(thiserror::Error, Debug)]
pub enum ClientBuilderError {
    #[error("Invalid Header Value")]
    InvalidHeaderValue(#[from] header::InvalidHeaderValue),
    #[error("Could not construct client from builder")]
    BuilderError(#[from] reqwest::Error),
}

pub fn client(api_token: &str) -> Result<reqwest::Client, ClientBuilderError> {
    let mut headers = header::HeaderMap::new();
    let mut authorization = header::HeaderValue::from_str(&format!("Token token={api_token}"))?;
    authorization.set_sensitive(true);
    headers.insert(
        header::ACCEPT,
        header::HeaderValue::from_static("application/json"),
    );
    headers.insert(header::AUTHORIZATION, authorization);

    Ok(reqwest::ClientBuilder::new()
        .default_headers(headers)
        .build()?)
}

/// Builder for construction a Tito API Client
pub struct ClientBuilder {
    client: reqwest::Client,
    base_url: Option<String>,
}

impl ClientBuilder {
    pub fn new(api_token: &str) -> Result<Self, ClientBuilderError> {
        Ok(Self {
            client: client(&api_token)?,
            base_url: None,
        })
    }

    pub fn base_url<'a>(&'a mut self, url: impl Into<String>) -> &'a ClientBuilder {
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

/// Tito API Client
#[derive(Clone)]
pub struct Client {
    client: reqwest::Client,
    base_url: String,
}

impl<'a> Client {
    pub fn tickets(&'a self, account_slug: &str, event_slug: &str) -> TicketsHandler<'a> {
        TicketsHandler::new(self, account_slug, event_slug)
    }
}
