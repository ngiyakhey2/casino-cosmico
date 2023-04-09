use crate::tito::admin::meta::Meta;
use chrono::{offset::Utc, DateTime};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Ticket {
    pub _type: String,
    pub id: u32,
    pub slug: String,
    pub unique_url: String,
    pub company_name: Option<String>,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub number: u32,
    pub price: f32,
    pub reference: String,
    pub state: State,
    pub test_mode: bool,
    pub registration_id: u32,
    pub registration_name: String,
    pub registration_email: String,
    pub release_id: u32,
    pub release_title: String,
    pub release_archived: bool,
    pub avatar_url: String,
    pub void: bool,
    pub changes_locked: bool,
    pub consented_at: Option<DateTime<Utc>>,
    pub discounted_code_used: Option<String>,
    pub tag_names: Vec<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct Tickets {
    pub tickets: Vec<Ticket>,
    pub meta: Meta,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum State {
    New,
    Complete,
    Incomplete,
    Reminder,
    Void,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticket_deserializes() {
        let ticket: Result<Ticket, _> =
            serde_json::from_str(include_str!("../../../fixtures/ticket.json"));

        assert!(ticket.is_ok());
    }
}
