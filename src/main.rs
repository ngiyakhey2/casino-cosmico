use casino_cosmico::tito;

const EARLY_BIRD_TICKET_SLUG: &str = "con-of-the-rings-early-bird-ticket";

#[tokio::main]
async fn main() {
    let tito_api_token = std::env::var("TITO_API_TOKEN").unwrap();

    let client = tito::client::ClientBuilder::new(&tito_api_token)
        .expect("Could not build Tito HTTP Client")
        .build();
    let tickets = client
        .tickets("con-of-heroes", "con-of-heroes")
        .release_ids(vec![EARLY_BIRD_TICKET_SLUG.to_string()])
        .send()
        .await
        .unwrap();

    println!("{:?}", tickets.len());
}
