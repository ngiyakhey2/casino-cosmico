use crate::tito::admin::{
    client::Client,
    ticket::{Ticket, Tickets},
};
use chrono::{offset::Utc, DateTime};

#[derive(strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum State {
    Complete,
    Incomplete,
    Unassigned,
    Void,
    ChangesAllowed,
    ChangesLocked,
    Archived,
}

#[derive(strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Type {
    Manual,
    Standard,
}

#[derive(strum::Display)]
pub enum Direction {
    #[strum(serialize = "asc")]
    Ascending,
    #[strum(serialize = "desc")]
    Descending,
}

pub struct FilterDate {
    pub operator: Operator,
    pub date_time: DateTime<Utc>,
}

#[derive(strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Operator {
    Gt,
    Gte,
    Lt,
    Lte,
}

/// Client to Tito's Tickets API
pub struct TicketsHandler<'client> {
    client: &'client Client,
    account: String,
    event: String,
    sort: Option<String>,
    direction: Option<Direction>,
    states: Option<Vec<State>>,
    types: Option<Vec<Type>>,
    activity_ids: Option<Vec<u32>>,
    release_ids: Option<Vec<String>>,
    created_at: Option<Vec<FilterDate>>,
    updated_at: Option<Vec<FilterDate>>,
}

impl<'client> TicketsHandler<'client> {
    pub(crate) fn new(
        client: &'client Client,
        account: impl Into<String>,
        event: impl Into<String>,
    ) -> Self {
        Self {
            client,
            account: account.into(),
            event: event.into(),
            sort: None,
            direction: None,
            states: None,
            types: None,
            activity_ids: None,
            release_ids: None,
            created_at: None,
            updated_at: None,
        }
    }

    pub fn sort(mut self, sort: impl Into<String>) -> Self {
        self.sort = Some(sort.into());
        self
    }

    pub fn states(mut self, states: Vec<State>) -> Self {
        self.states = Some(states);
        self
    }

    pub fn types(mut self, types: Vec<Type>) -> Self {
        self.types = Some(types);
        self
    }

    pub fn activity_ids(mut self, activity_ids: Vec<u32>) -> Self {
        self.activity_ids = Some(activity_ids);
        self
    }

    pub fn release_ids(mut self, release_ids: Vec<String>) -> Self {
        self.release_ids = Some(release_ids);
        self
    }

    pub fn created_at(mut self, created_at: Vec<FilterDate>) -> Self {
        self.created_at = Some(created_at);
        self
    }

    pub fn updated_at(mut self, updated_at: Vec<FilterDate>) -> Self {
        self.updated_at = Some(updated_at);
        self
    }

    /// Execute the request to fetch all tickets
    pub async fn send(&self) -> Result<Vec<Ticket>, reqwest::Error> {
        let mut next_page = Some(1);
        let mut tickets: Vec<Ticket> = Vec::new();
        while let Some(page) = next_page {
            let mut response = self.build(page).send().await?.json::<Tickets>().await?;

            next_page = response.meta.next_page;
            tickets.append(&mut response.tickets);
        }

        Ok(tickets)
    }

    /// Construct RequestBuilder for pagination. RequestBuilder doesn't support Clone.
    fn build(&self, page: u32) -> reqwest::RequestBuilder {
        let mut request_builder = self.client.client.get(format!(
            "{}/{}/{}/tickets",
            self.client.base_url, self.account, self.event
        ));

        if let Some(sort) = &self.sort {
            request_builder = request_builder.query(&[("search[sort]", sort)]);
        }

        if let Some(direction) = &self.direction {
            request_builder =
                request_builder.query(&[("search[direction]", &direction.to_string())]);
        }

        if let Some(states) = &self.states {
            for state in states {
                request_builder =
                    request_builder.query(&[("search[states][]", &state.to_string())]);
            }
        }

        if let Some(types) = &self.types {
            for r#type in types {
                request_builder = request_builder.query(&[("search[types][]", r#type.to_string())]);
            }
        }

        if let Some(activity_ids) = &self.activity_ids {
            for activity_id in activity_ids {
                request_builder = request_builder.query(&[("search[activity_ids][]", activity_id)]);
            }
        }

        if let Some(release_ids) = &self.release_ids {
            for release_id in release_ids {
                request_builder = request_builder.query(&[("search[release_ids][]", release_id)]);
            }
        }

        if let Some(created_at) = &self.created_at {
            for date in created_at {
                request_builder = request_builder.query(&[(
                    format!("?search[created_at][{}]", date.operator),
                    date.date_time.to_rfc3339(),
                )]);
            }
        }

        if let Some(updated_at) = &self.updated_at {
            for date in updated_at {
                request_builder = request_builder.query(&[(
                    format!("?search[updated_at][{}]", date.operator),
                    date.date_time.to_rfc3339(),
                )]);
            }
        }

        request_builder.query(&[("page", page)])
    }
}
