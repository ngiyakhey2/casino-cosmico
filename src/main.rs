use casino_cosmico::tito;
use redis::AsyncCommands;

const ACCOUNT_SLUG: &str = "con-of-heroes";
const EVENT_SLUG: &str = "con-of-heroes";
const EARLY_BIRD_TICKET_SLUG: &str = "con-of-the-rings-early-bird-ticket";

#[tokio::main]
async fn main() {
    let tito_api_token = std::env::var("TITO_API_TOKEN").unwrap();
    let mut rediss_url = url::Url::parse(&std::env::var("REDIS_TLS_URL").unwrap()).unwrap();
    // Heroku Redis uses self signed certs, so need to set OPENSSL_VERIFY_NONE
    // https://devcenter.heroku.com/articles/heroku-redis#security-and-compliance
    rediss_url.set_fragment(Some("insecure"));

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

    let redis_client = redis::Client::open(rediss_url).expect("Could not connect to redis.");
    let mut connection = redis_client.get_tokio_connection().await.unwrap();
    let _: () = connection.rpush("raffle", attendees).await.unwrap();

    let redis_tickets: Vec<String> = connection.lrange("raffle", 0, -1).await.unwrap();
    let _: () = connection.del("raffle").await.unwrap();

    println!("{:?}", tickets.len());
    println!("{:?}", redis_tickets);
}
