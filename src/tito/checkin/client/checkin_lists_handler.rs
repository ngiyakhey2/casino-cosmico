use crate::tito::checkin::client::Client;
use chrono::{offset::Utc, DateTime};
use serde::Deserialize;

pub struct CheckinListsHandler<'client> {
    client: &'client Client,
    checkin_list_slug: String,
}

impl<'client> CheckinListsHandler<'client> {
    pub(crate) fn new(client: &'client Client, checkin_list_slug: impl Into<String>) -> Self {
        Self {
            client,
            checkin_list_slug: checkin_list_slug.into(),
        }
    }

    pub fn tickets(&'client self) -> TicketsHandler<'client> {
        TicketsHandler::new(self)
    }
}

#[derive(Deserialize)]
pub struct Ticket {
    pub id: u32,
    pub slug: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub company_name: Option<String>,
    pub release_title: String,
    pub reference: String,
    pub registration_reference: String,
    pub tags: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct TicketsHandler<'a> {
    checkin_lists_handler: &'a CheckinListsHandler<'a>,
}

impl<'a> TicketsHandler<'a> {
    pub(crate) fn new(checkin_lists_handler: &'a CheckinListsHandler) -> Self {
        Self {
            checkin_lists_handler,
        }
    }

    pub async fn send(&self) -> Result<Vec<Ticket>, reqwest::Error> {
        let response = self.build().send().await?.json::<Vec<Ticket>>().await?;

        Ok(response)
    }

    fn build(&self) -> reqwest::RequestBuilder {
        self.checkin_lists_handler.client.client.get(format!(
            "{}/checkin_lists/{}/tickets",
            self.checkin_lists_handler.client.base_url,
            self.checkin_lists_handler.checkin_list_slug
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticket_deserializes() {
        let ticket: Result<Ticket, _> =
            serde_json::from_str(include_str!("../../../../fixtures/checkin/ticket.json"));

        assert!(ticket.is_ok());
    }
}
