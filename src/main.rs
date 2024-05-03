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
    builder::{CreateCommand, CreateCommandOption, CreateMessage},
    client::{Context, EventHandler},
    http::Http,
    model::{
        application::{CommandDataOptionValue, CommandInteraction, CommandOptionType, Interaction},
        channel::{Reaction, ReactionType},
        gateway::{GatewayIntents, Ready},
        id::{ApplicationId, ChannelId, GuildId},
    },
    prelude::RwLock,
    Error as SerenityError,
};
use std::{env, sync::Arc};
use tracing::{error, info, instrument};

const EARLY_BIRD_TICKET_SLUG: &str = "Con of Heroes 2024 Early Bird Ticket";
const GENERAL_TICKET_SLUG: &str = "Con of heroes 2024 General Ticket";
const NO_SWAG_TICKET_SLUG: &str = "No-SWAG ticket";
const TICKET_SPOOFER_SLUG: &str = "Ticket spoofer";
const LOADED_REDIS_KEY: &str = "loaded";
const RAFFLE_REDIS_KEY: &str = "raffle";

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
        guild_id
            .create_command(
                &ctx.http,
                CreateCommand::new("raffle")
                    .description("Raffle Subcommand")
                    .add_option(
                        CreateCommandOption::new(
                            CommandOptionType::SubCommand,
                            "pick",
                            "Pick a winner",
                        )
                        .add_sub_option(
                            CreateCommandOption::new(
                                CommandOptionType::Integer,
                                "amount",
                                "Number of winners to pick",
                            )
                            .min_int_value(1),
                        ),
                    )
                    .add_option(
                        CreateCommandOption::new(
                            CommandOptionType::SubCommand,
                            "add",
                            "Add an entry by hand",
                        )
                        .add_sub_option(
                            CreateCommandOption::new(
                                CommandOptionType::String,
                                "name",
                                "Entry's Full Name",
                            )
                            .required(true),
                        ),
                    )
                    .add_option(CreateCommandOption::new(
                        CommandOptionType::SubCommand,
                        "clear",
                        "Clear raffle list",
                    ))
                    .add_option(CreateCommandOption::new(
                        CommandOptionType::SubCommand,
                        "load",
                        "Load tickets from tito",
                    ))
                    .add_option(CreateCommandOption::new(
                        CommandOptionType::SubCommand,
                        "size",
                        "Number of entries in the raffle",
                    )),
            )
            .await
            .unwrap();
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            if command.data.name.as_str() != "raffle" {
                return;
            }

            if let Err(err) = match_subcommand(&ctx, &command).await {
                error!("Cannot respond to slash comamnd: {}", err);
            }
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
                            commands::add_name(&ctx, LOADED_REDIS_KEY, RAFFLE_REDIS_KEY, name)
                                .await
                                .unwrap();
                            channel_id
                                .send_message(
                                    &ctx.http,
                                    CreateMessage::new().content(format!("Re-adding **{name}**")),
                                )
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
    command: &CommandInteraction,
) -> Result<(), SlashCommandError> {
    let sub_cmd = command
        .data
        .options
        .get(0)
        .ok_or(SlashCommandError::NoSubCommand)?;
    if let CommandDataOptionValue::SubCommand(options) = &sub_cmd.value {
        match sub_cmd.name.as_str() {
            "add" => {
                if let Some(option) = options.get(0) {
                    if let CommandDataOptionValue::String(name) = &option.value {
                        return commands::add(
                            ctx,
                            command,
                            LOADED_REDIS_KEY,
                            RAFFLE_REDIS_KEY,
                            name,
                        )
                        .await
                        .map_err(|err| err.into());
                    }
                }

                Err(SlashCommandError::MissingOption(
                    "add".into(),
                    "name".into(),
                ))
            }
            "clear" => commands::clear(ctx, command, LOADED_REDIS_KEY, RAFFLE_REDIS_KEY)
                .await
                .map_err(|err| err.into()),
            "load" => {
                let load_params = commands::LoadParams {
                    checkin_list_slug: &type_map_keys::CheckinListSlug::get(&ctx.data).await,
                    loaded_redis_key: LOADED_REDIS_KEY,
                    raffle_redis_key: RAFFLE_REDIS_KEY,
                    ticket_slugs: [
                        EARLY_BIRD_TICKET_SLUG,
                        GENERAL_TICKET_SLUG,
                        NO_SWAG_TICKET_SLUG,
                        TICKET_SPOOFER_SLUG,
                    ]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                };
                commands::load(ctx, command, load_params)
                    .await
                    .map_err(|err| err.into())
            }
            "pick" => {
                let amount: u64 = *options
                    .get(0)
                    .map(|option| match &option.value {
                        CommandDataOptionValue::Integer(i) => Ok(i),
                        _ => Err(SlashCommandError::UnknownSubCommand),
                    })
                    .unwrap_or(Ok(&1))? as u64;
                commands::raffle(ctx, command, RAFFLE_REDIS_KEY, amount)
                    .await
                    .map_err(|err| err.into())
            }
            "size" => commands::size(ctx, command, RAFFLE_REDIS_KEY)
                .await
                .map_err(|err| err.into()),
            _ => Err(SlashCommandError::UnknownSubCommand),
        }
    } else {
        Err(SlashCommandError::UnknownSubCommand)
    }
}

#[tokio::main]
#[instrument]
async fn main() {
    tracing_subscriber::fmt::init();

    let checkin_list_slug =
        env::var("CHECKIN_LIST_SLUG").expect("Expected env variable: CHECKIN_LIST_SLUG");
    let redis_url = env::var("REDIS_TLS_URL").expect("Expected env variable: REDIS_TLS_URL");
    let discord_token = env::var("DISCORD_TOKEN").expect("Expected env variable: DISCORD_TOKEN");
    let guild_id = GuildId::new(
        env::var("DISCORD_GUILD_ID")
            .expect("Expected env variable: DISCORD_GUILD_ID")
            .parse()
            .expect("DISCORD_GUILD_ID must be an integer"),
    );
    let application_id = ApplicationId::new(
        env::var("DISCORD_APPLICATION_ID")
            .expect("Expected environment variable: DISCORD_APPLICATION_ID")
            .parse()
            .expect("application id is not a valid id"),
    );
    let channel_id = ChannelId::new(
        env::var("DISCORD_CHANNEL_ID")
            .expect("Expected environment variable: DISCORD_CHANNEL_ID")
            .parse()
            .expect("channel id is not a valid id"),
    );
    let connection = redis_pool(&redis_url).await.unwrap();
    let tito_client = tito::checkin::client::ClientBuilder::new()
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
        data.insert::<type_map_keys::CheckinListSlug>(checkin_list_slug);
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
