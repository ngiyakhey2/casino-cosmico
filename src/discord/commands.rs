use crate::discord::type_map_keys;
use crate::tito::checkin::client::Client;
use bb8_redis::redis::AsyncCommands;
use serenity::{
    client::Context,
    model::application::interaction::{
        application_command::ApplicationCommandInteraction, InteractionResponseType,
    },
};
use std::collections::HashSet;
use tracing::instrument;

#[derive(Debug)]
pub struct LoadParams<'a> {
    pub checkin_list_slug: &'a str,
    pub loaded_redis_key: &'a str,
    pub raffle_redis_key: &'a str,
    pub ticket_slugs: Vec<String>,
}

#[instrument(skip(ctx))]
pub async fn load<'a>(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    params: LoadParams<'a>,
) -> serenity::Result<()> {
    let tito_client = type_map_keys::TitoClient::get(&ctx.data).await;
    let redis_pool = type_map_keys::RedisPool::get(&ctx.data).await;

    let (loaded, total) = load_names(&tito_client, &redis_pool, params).await?;

    command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message.content(format!("Loaded {loaded} users\n{total} total users.",))
                })
        })
        .await
}

async fn load_names<'a>(
    tito_client: &Client,
    redis_pool: &bb8::Pool<bb8_redis::RedisConnectionManager>,
    params: LoadParams<'a>,
) -> serenity::Result<(usize, usize)> {
    let mut redis_connection = redis_pool.get().await.unwrap();

    let (checkins, tickets) = futures::future::try_join(
        tito_client
            .check_ins(params.checkin_list_slug)
            .checkins()
            .send(),
        tito_client
            .check_ins(params.checkin_list_slug)
            .tickets()
            .send(),
    )
    .await
    .unwrap();
    let checkins_hash: HashSet<u32> =
        HashSet::from_iter(checkins.iter().map(|checkin| checkin.ticket_id));

    let already_loaded: Vec<String> = redis_connection
        .smembers(params.loaded_redis_key)
        .await
        .unwrap();
    let attendees = tickets
        .iter()
        .filter_map(|ticket| {
            if params.ticket_slugs.contains(&ticket.release_title)
                && checkins_hash.get(&ticket.id).is_some()
            {
                if let Some(first_name) = &ticket.first_name {
                    if let Some(last_name) = &ticket.last_name {
                        let name = format!("{first_name} {last_name}");
                        if !already_loaded.contains(&name) {
                            return Some(name);
                        }
                    }
                }
            }

            None
        })
        .collect::<Vec<String>>();
    let unique_attendees = HashSet::<String>::from_iter(attendees);
    // this will error with an empty set
    if !unique_attendees.is_empty() {
        let _: () = redis_connection
            .rpush(params.raffle_redis_key, &unique_attendees)
            .await
            .unwrap();
        let _: () = redis_connection
            .sadd(params.loaded_redis_key, &unique_attendees)
            .await
            .unwrap();
    }
    let loaded: Vec<String> = redis_connection
        .lrange(params.raffle_redis_key, 0, -1)
        .await
        .unwrap();

    Ok((unique_attendees.len(), loaded.len()))
}

#[instrument(skip(ctx))]
pub async fn raffle(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    redis_key: &str,
    amount: u64,
) -> serenity::Result<()> {
    let redis_pool = type_map_keys::RedisPool::get(&ctx.data).await;
    let mut redis_connection = redis_pool.get().await.unwrap();

    let size: usize = redis_connection.llen(redis_key).await.unwrap();
    let entries = std::cmp::min(size, amount as usize);

    match entries {
        0..=1 => {
            // will always return 1, since we check size before this
            if let Some(winner) = pick_winner(&redis_pool, redis_key, ctx).await {
                command
                    .create_interaction_response(&ctx.http, |response| {
                        response
                            .kind(InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|message| {
                                message.content(format!("Winner is **{winner}**"))
                            })
                    })
                    .await?;
            } else {
                command
                    .create_interaction_response(&ctx.http, |response| {
                        response
                            .kind(InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|message| {
                                message.content("No entries in the raffle.")
                            })
                    })
                    .await?;
            }
        }
        _ => {
            command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| {
                            message.content(format!("Found {entries} winners"))
                        })
                })
                .await?;

            for _ in 0..amount {
                if let Some(winner) = pick_winner(&redis_pool, redis_key, ctx).await {
                    command
                        .channel_id
                        .send_message(&ctx.http, |m| m.content(format!("Winner: **{winner}**")))
                        .await?;
                } else {
                    break;
                }
            }
        }
    }

    Ok(())
}

async fn pick_winner(
    redis_pool: &bb8::Pool<bb8_redis::RedisConnectionManager>,
    raffle_redis_key: &str,
    ctx: &Context,
) -> Option<String> {
    let mut redis_connection = redis_pool.get().await.unwrap();
    let size: usize = redis_connection.llen(raffle_redis_key).await.unwrap();

    if size == 0 {
        return None;
    }

    let index: isize = type_map_keys::Rng::rand(&ctx.data, size).await as isize;
    let winner: String = redis_connection
        .lindex(raffle_redis_key, index)
        .await
        .unwrap();
    let _: () = redis_connection
        .lrem(raffle_redis_key, 1, &winner)
        .await
        .unwrap();

    Some(winner)
}

#[instrument(skip(ctx))]
pub async fn clear(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    loaded_redis_key: &str,
    raffle_redis_key: &str,
) -> serenity::Result<()> {
    let redis_pool = type_map_keys::RedisPool::get(&ctx.data).await;
    let mut redis_connection = redis_pool.get().await.unwrap();
    let _: () = redis_connection.del(loaded_redis_key).await.unwrap();
    let _: () = redis_connection.del(raffle_redis_key).await.unwrap();

    command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.content("Cleared list"))
        })
        .await
}

#[instrument(skip(ctx))]
pub async fn add(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    loaded_redis_key: &str,
    raffle_redis_key: &str,
    name: &str,
) -> serenity::Result<()> {
    add_name(ctx, loaded_redis_key, raffle_redis_key, name).await?;

    command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.content(format!("Added {name}")))
        })
        .await
}

#[instrument(skip(ctx))]
pub async fn add_name(
    ctx: &Context,
    loaded_redis_key: &str,
    raffle_redis_key: &str,
    name: &str,
) -> serenity::Result<()> {
    let redis_pool = type_map_keys::RedisPool::get(&ctx.data).await;
    let mut redis_connection = redis_pool.get().await.unwrap();
    let _: () = redis_connection.sadd(loaded_redis_key, name).await.unwrap();
    let _: () = redis_connection
        .rpush(raffle_redis_key, name)
        .await
        .unwrap();

    Ok(())
}

#[instrument(skip(ctx))]
pub async fn size(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    raffle_redis_key: &str,
) -> serenity::Result<()> {
    let redis_pool = type_map_keys::RedisPool::get(&ctx.data).await;
    let mut redis_connection = redis_pool.get().await.unwrap();
    let size: usize = redis_connection.llen(raffle_redis_key).await.unwrap();

    command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message.content(format!("{size} entries in the raffle"))
                })
        })
        .await
}
