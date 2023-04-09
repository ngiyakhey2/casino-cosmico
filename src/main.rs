use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use casino_cosmico::{
    discord::{commands, type_map_keys},
    tito,
};
use lazy_static::lazy_static;
use rand::SeedableRng;
use regex::Regex;
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    http::client::Http,
    model::{
        application::{
            command::CommandOptionType,
            interaction::{application_command::ApplicationCommandInteraction, Interaction},
        },
        channel::{Reaction, ReactionType},
        gateway::{GatewayIntents, Ready},
        id::ChannelId,
        prelude::GuildId,
    },
    prelude::RwLock,
    Error as SerenityError,
};
use std::{env, sync::Arc};
use tracing::{error, info, instrument};

const ACCOUNT_SLUG: &str = "con-of-heroes";
const EVENT_SLUG: &str = "con-of-heroes-2022";
const EARLY_BIRD_TICKET_SLUG: &str = "con-of-the-rings-early-bird-ticket";
const GENERAL_TICKET_SLUG: &str = "con-of-heroes-general-ticket";
const REDIS_KEY: &str = "raffle";

/// Setup and return an async redis pool
async fn redis_pool(redis_str: &str) -> Result<Pool<RedisConnectionManager>, redis::RedisError> {
    // Heroku Redis uses self signed certs, so need to set OPENSSL_VERIFY_NONE
    // https://devcenter.heroku.com/articles/heroku-redis#security-and-compliance
    let mut url = url::Url::parse(redis_str).unwrap();
    url.set_fragment(Some("insecure"));

    let manager = RedisConnectionManager::new(url)?;
    Pool::builder().build(manager).await
}

#[derive(thiserror::Error, Debug)]
enum SlashCommandError {
    #[error("Missing Option {0} for {1}")]
    MissingOption(String, String),
    #[error("No Sub-Command Provided")]
    NoSubCommand,
    #[error("Unknown Sub-Command")]
    UnknownSubCommand,
    #[error("Serenity Error: {0}")]
    Serenity(#[from] SerenityError),
}

struct SlashHandler;

#[async_trait]
impl EventHandler for SlashHandler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        let guild_id = type_map_keys::GuildId::get(&ctx.data).await;
        GuildId::set_application_commands(&guild_id, &ctx.http, |commands| {
            commands.create_application_command(|command| {
                command
                    .name("raffle")
                    .description("Raffle Subcommand")
                    .create_option(|option| {
                        option
                            .name("pick")
                            .description("Pick a winner")
                            .kind(CommandOptionType::SubCommand)
                            .default_option(true)
                            .create_sub_option(|option| {
                                option
                                    .name("amount")
                                    .description("Number of winners to pick")
                                    .kind(CommandOptionType::Integer)
                                    .min_int_value(1)
                            })
                    })
                    .create_option(|option| {
                        option
                            .name("add")
                            .description("Add an entry by hand")
                            .kind(CommandOptionType::SubCommand)
                            .create_sub_option(|option| {
                                option
                                    .name("name")
                                    .description("Entry's Full Name")
                                    .kind(CommandOptionType::String)
                                    .required(true)
                            })
                    })
                    .create_option(|option| {
                        option
                            .name("clear")
                            .description("Clear raffle list")
                            .kind(CommandOptionType::SubCommand)
                    })
                    .create_option(|option| {
                        option
                            .name("load")
                            .description("Load tickets from tito")
                            .kind(CommandOptionType::SubCommand)
                    })
                    .create_option(|option| {
                        option
                            .name("size")
                            .description("Number of entries in the raffle")
                            .kind(CommandOptionType::SubCommand)
                    })
            })
        })
        .await
        .unwrap();
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let command = match interaction.application_command() {
            Some(c) => c,
            None => return,
        };

        if command.data.name.as_str() != "raffle" {
            return;
        }

        if let Err(err) = match_subcommand(&ctx, &command).await {
            error!("Cannot respond to slash comamnd: {}", err);
        }
    }

    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        let channel_id = type_map_keys::ChannelId::get(&ctx.data).await;
        let user_id = type_map_keys::UserId::get(&ctx.data).await;

        if reaction.channel_id == channel_id {
            let message = reaction.message(&ctx.http).await.unwrap();
            if message.author.id == user_id {
                if let ReactionType::Unicode(ref code) = reaction.emoji {
                    if code == "👍" {
                        lazy_static! {
                            static ref RE: Regex =
                                Regex::new(r"[*]{2}(?P<name>[^*]+)[*]{2}").unwrap();
                        }

                        let contents = message.content;
                        if let Some(caps) = RE.captures(&contents) {
                            let name = &caps["name"];
                            commands::add_name(&ctx, REDIS_KEY, &name).await.unwrap();
                            channel_id
                                .send_message(&ctx.http, |m| {
                                    m.content(format!("Re-adding **{name}**"))
                                })
                                .await
                                .unwrap();
                        }
                    }
                }
            }
        }
    }
}

/// Maps Slash Sub-Commands to function calls
async fn match_subcommand(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
) -> Result<(), SlashCommandError> {
    let sub_cmd = command
        .data
        .options
        .get(0)
        .ok_or(SlashCommandError::NoSubCommand)?;
    match sub_cmd.name.as_str() {
        "add" => {
            if let Some(name) = sub_cmd
                .options
                .get(0)
                .and_then(|option| option.value.as_ref())
                .and_then(|value| value.as_str())
            {
                commands::add(ctx, command, REDIS_KEY, name)
                    .await
                    .map_err(|err| err.into())
            } else {
                Err(SlashCommandError::MissingOption(
                    "add".into(),
                    "name".into(),
                ))
            }
        }
        "clear" => commands::clear(ctx, command, REDIS_KEY)
            .await
            .map_err(|err| err.into()),
        "load" => {
            let load_params = commands::LoadParams {
                account_slug: ACCOUNT_SLUG,
                event_slug: EVENT_SLUG,
                redis_key: REDIS_KEY,
                ticket_slugs: [EARLY_BIRD_TICKET_SLUG, GENERAL_TICKET_SLUG]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            };
            commands::load(ctx, command, load_params)
                .await
                .map_err(|err| err.into())
        }
        "pick" => {
            let amount = sub_cmd
                .options
                .get(0)
                .and_then(|option| option.value.as_ref())
                .and_then(|value| value.as_u64())
                .unwrap_or(1);
            commands::raffle(ctx, command, REDIS_KEY, amount)
                .await
                .map_err(|err| err.into())
        }
        "size" => commands::size(ctx, command, REDIS_KEY)
            .await
            .map_err(|err| err.into()),
        _ => Err(SlashCommandError::UnknownSubCommand),
    }
}

#[tokio::main]
#[instrument]
async fn main() {
    tracing_subscriber::fmt::init();

    let tito_api_token =
        env::var("TITO_API_TOKEN").expect("Expected environment variable: TITO_API_TOKEN");
    let redis_url = env::var("REDIS_TLS_URL").expect("Expected env variable: REDIS_TLS_URL");
    let discord_token = env::var("DISCORD_TOKEN").expect("Expected env variable: DISCORD_TOKEN");
    let guild_id = GuildId(
        env::var("DISCORD_GUILD_ID")
            .expect("Expected env variable: DISCORD_GUILD_ID")
            .parse()
            .expect("DISCORD_GUILD_ID must be an integer"),
    );
    let application_id: u64 = env::var("DISCORD_APPLICATION_ID")
        .expect("Expected environment variable: DISCORD_APPLICATION_ID")
        .parse()
        .expect("application id is not a valid id");
    let channel_id = ChannelId(
        env::var("DISCORD_CHANNEL_ID")
            .expect("Expected environment variable: DISCORD_CHANNEL_ID")
            .parse()
            .expect("channel id is not a valid id"),
    );
    let connection = redis_pool(&redis_url).await.unwrap();
    let tito_client = tito::admin::client::ClientBuilder::new(&tito_api_token)
        .expect("Could not build Tito HTTP Client")
        .build();
    let rng = Arc::new(RwLock::new(rand::rngs::StdRng::from_entropy()));

    let http = Http::new(&discord_token);
    let bot_id = match http.get_current_user().await {
        Ok(info) => info.id,
        Err(why) => panic!("Could not access user info: {:?}", why),
    };

    let gateway_intents = GatewayIntents::GUILD_MESSAGE_REACTIONS;
    let mut client = serenity::Client::builder(discord_token, gateway_intents)
        .application_id(application_id)
        .event_handler(SlashHandler)
        .await
        .expect("Error creating Discord cliet.");

    {
        let mut data = client.data.write().await;
        data.insert::<type_map_keys::ChannelId>(channel_id);
        data.insert::<type_map_keys::GuildId>(guild_id);
        data.insert::<type_map_keys::UserId>(bot_id);
        data.insert::<type_map_keys::RedisPool>(connection);
        data.insert::<type_map_keys::TitoClient>(tito_client);
        data.insert::<type_map_keys::Rng>(rng);
    }

    if let Err(err) = client.start().await {
        error!("Client error: {:?}", err);
    }
}
