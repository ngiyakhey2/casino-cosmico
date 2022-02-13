pub mod discord;
pub mod tito;

use reqwest::{header, ClientBuilder};

pub async fn tito_test() {
    let mut headers = header::HeaderMap::new();
    let tito_api_token = std::env::var("TITO_API_TOKEN").unwrap();
    let mut authorization =
        header::HeaderValue::from_str(&format!("Token token={tito_api_token}")).unwrap();
    authorization.set_sensitive(true);
    headers.insert(
        header::ACCEPT,
        header::HeaderValue::from_static("application/json"),
    );
    headers.insert(header::AUTHORIZATION, authorization);

    let client = ClientBuilder::new()
        .default_headers(headers)
        .build()
        .unwrap();
    let resp = client
        .get("https://api.tito.io/v3/hello")
        .send()
        .await
        .unwrap();

    println!("{:?}", resp.text().await.unwrap());
}
