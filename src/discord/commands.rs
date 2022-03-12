use crate::{discord::type_map_keys, tito};
use bb8_redis::redis::AsyncCommands;
use serenity::{
    client::Context,
    model::interactions::{
        application_command::ApplicationCommandInteraction, InteractionResponseType,
    },
};

pub async fn pong(ctx: &Context, command: &ApplicationCommandInteraction) -> serenity::Result<()> {
    command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.content("Pong"))
        })
        .await
}

pub struct LoadParams<'a> {
    pub account_slug: &'a str,
    pub event_slug: &'a str,
    pub redis_key: &'a str,
    pub ticket_slugs: Vec<String>,
}

pub async fn load<'a>(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    params: LoadParams<'a>,
) -> serenity::Result<()> {
    let data = ctx.data.read().await;
    let redis_pool = data
        .get::<type_map_keys::RedisPool>()
        .expect("Expected RedisPool in TypeMap");
    let tito_client = data
        .get::<type_map_keys::TitoClient>()
        .expect("Expected TitoClient in TypeMap");
    let mut redis_connection = redis_pool.get().await.unwrap();

    let tickets = tito_client
        .tickets(params.account_slug, params.event_slug)
        .release_ids(params.ticket_slugs)
        .states(vec![tito::client::tickets_handler::State::Complete])
        .send()
        .await
        .unwrap();
    let attendees = tickets
        .iter()
        .filter_map(|ticket| {
            if let Some(first_name) = &ticket.first_name {
                if let Some(last_name) = &ticket.last_name {
                    return Some(format!("{first_name} {last_name}"));
                }
            }

            return None;
        })
        .collect::<Vec<String>>();
    let _: () = redis_connection
        .rpush(params.redis_key, &attendees)
        .await
        .unwrap();
    let loaded: Vec<String> = redis_connection
        .lrange(params.redis_key, 0, -1)
        .await
        .unwrap();

    command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message.content(format!(
                        "Loaded {} users\n{} total users.",
                        &attendees.len(),
                        &loaded.len()
                    ))
                })
        })
        .await
}
