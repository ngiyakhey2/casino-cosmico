use casino_cosmico::tito;
use redis::AsyncCommands;
use std::env;

const ACCOUNT_SLUG: &str = "con-of-heroes";
const EVENT_SLUG: &str = "con-of-heroes";
const EARLY_BIRD_TICKET_SLUG: &str = "con-of-the-rings-early-bird-ticket";

/// Setup and return an async redis connection
async fn redis_connection(redis_str: &str) -> Result<redis::aio::Connection, redis::RedisError> {
    // Heroku Redis uses self signed certs, so need to set OPENSSL_VERIFY_NONE
    // https://devcenter.heroku.com/articles/heroku-redis#security-and-compliance
    let mut url = url::Url::parse(redis_str).unwrap();
    url.set_fragment(Some("insecure"));

    let redis_client = redis::Client::open(url)?;
    redis_client.get_tokio_connection().await
}

#[tokio::main]
async fn main() {
    let tito_api_token = env::var("TITO_API_TOKEN").unwrap();
    let redis_url = env::var("REDIS_TLS_URL").unwrap();
    let discord_token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let application_id: u64 = env::var("DISCORD_APPLICATION_ID")
        .expect("Expected an application id in the environment")
        .parse()
        .expect("application id is not a valid id");

    let tito_client = tito::client::ClientBuilder::new(&tito_api_token)
        .expect("Could not build Tito HTTP Client")
        .build();
    let tickets = tito_client
        .tickets(ACCOUNT_SLUG, EVENT_SLUG)
        .release_ids(vec![EARLY_BIRD_TICKET_SLUG.to_string()])
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

    let mut connection = redis_connection(&redis_url).await.unwrap();
    let _: () = connection.rpush("raffle", attendees).await.unwrap();

    let redis_tickets: Vec<String> = connection.lrange("raffle", 0, -1).await.unwrap();
    let _: () = connection.del("raffle").await.unwrap();

    println!("{:?}", tickets.len());
    println!("{:?}", redis_tickets);
}
