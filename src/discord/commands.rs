use crate::{discord::type_map_keys, tito};
use bb8_redis::redis::AsyncCommands;
use rand::Rng;
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
    let (redis_pool, tito_client) = {
        // keep read locks open as small as possible
        let data = ctx.data.read().await;

        (
            data.get::<type_map_keys::RedisPool>()
                .expect("Expected RedisPool in TypeMap")
                .clone(),
            data.get::<type_map_keys::TitoClient>()
                .expect("Expected TitoClient in TypeMap")
                .clone(),
        )
    };
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

pub async fn raffle(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    redis_key: &str,
) -> serenity::Result<()> {
    let redis_pool = {
        // keep read locks open as small as possible
        let data = ctx.data.read().await;
        data.get::<type_map_keys::RedisPool>()
            .expect("Expected RedisPool in TypeMap")
            .clone()
    };
    let mut redis_connection = redis_pool.get().await.unwrap();
    let size: usize = redis_connection.llen(redis_key).await.unwrap();
    if size <= 0 {
        return command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message.content(format!("No entries in the raffle."))
                    })
            })
            .await;
    }

    let index: isize = {
        // keep read locks open as small as possible
        let data = ctx.data.read().await;
        let rng_lock = data
            .get::<type_map_keys::Rng>()
            .expect("Expected Rng in TypeMap");
        // keep write locks open as small as possible
        let mut rng = rng_lock.write().await;
        rng.gen_range(0..size) as isize
    };
    let winner: String = redis_connection.lindex(redis_key, index).await.unwrap();
    let _: () = redis_connection.lrem(redis_key, 1, &winner).await.unwrap();

    command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.content(format!("Winner is {winner}")))
        })
        .await
}

pub async fn clear(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    redis_key: &str,
) -> serenity::Result<()> {
    let redis_pool = {
        // keep read locks open as small as possible
        let data = ctx.data.read().await;
        data.get::<type_map_keys::RedisPool>()
            .expect("Expected RedisPool in TypeMap")
            .clone()
    };
    let mut redis_connection = redis_pool.get().await.unwrap();
    let _: () = redis_connection.del(redis_key).await.unwrap();

    command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.content(format!("Cleared list")))
        })
        .await
}
