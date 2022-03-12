use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use casino_cosmico::{
    discord::{commands, type_map_keys},
    tito,
};
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{gateway::Ready, interactions::Interaction, prelude::GuildId},
};
use std::env;

const ACCOUNT_SLUG: &str = "con-of-heroes";
const EVENT_SLUG: &str = "con-of-heroes";
const EARLY_BIRD_TICKET_SLUG: &str = "con-of-the-rings-early-bird-ticket";
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

struct SlashHandler;

#[async_trait]
impl EventHandler for SlashHandler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        let data = ctx.data.read().await;
        let guild_id = data
            .get::<type_map_keys::GuildId>()
            .expect("Expected GuildId in TypeMap");

        let commands = GuildId::set_application_commands(&guild_id, &ctx.http, |commands| {
            commands
                .create_application_command(|command| {
                    command.name("ping").description("A ping command")
                })
                .create_application_command(|command| {
                    command.name("load").description("Load tickets from tito")
                })
        })
        .await;

        println!("Support the following Guild Commands: {:#?}", commands);
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let result = match command.data.name.as_str() {
                "ping" => commands::pong(&ctx, &command).await,
                "load" => {
                    let load_params = commands::LoadParams {
                        account_slug: ACCOUNT_SLUG,
                        event_slug: EVENT_SLUG,
                        redis_key: REDIS_KEY,
                        ticket_slugs: [EARLY_BIRD_TICKET_SLUG]
                            .iter()
                            .map(|s| s.to_string())
                            .collect(),
                    };
                    commands::load(&ctx, &command, load_params).await
                }
                _ => Ok(()),
            };

            if let Err(err) = result {
                eprintln!("Cannot respond to slash comamnd: {}", err);
            }
        }
    }
}

#[tokio::main]
async fn main() {
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
    let connection = redis_pool(&redis_url).await.unwrap();
    let tito_client = tito::client::ClientBuilder::new(&tito_api_token)
        .expect("Could not build Tito HTTP Client")
        .build();

    let mut client = serenity::Client::builder(discord_token)
        .application_id(application_id)
        .event_handler(SlashHandler)
        .await
        .expect("Error creating Discord cliet.");

    {
        let mut data = client.data.write().await;
        data.insert::<type_map_keys::GuildId>(guild_id);
        data.insert::<type_map_keys::RedisPool>(connection);
        data.insert::<type_map_keys::TitoClient>(tito_client);
    }

    if let Err(err) = client.start().await {
        eprintln!("Client error: {:?}", err);
    }
}
