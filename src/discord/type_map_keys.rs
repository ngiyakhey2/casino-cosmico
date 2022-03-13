//! Collection of Serenity TypeMapKeys
use crate::tito::client::Client;
use serenity::prelude::TypeMapKey;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct GuildId;

impl TypeMapKey for GuildId {
    type Value = serenity::model::prelude::GuildId;
}

pub struct RedisPool;
impl TypeMapKey for RedisPool {
    type Value = bb8::Pool<bb8_redis::RedisConnectionManager>;
}

pub struct TitoClient;
impl TypeMapKey for TitoClient {
    type Value = Client;
}

pub struct Rng;
impl TypeMapKey for Rng {
    type Value = Arc<RwLock<rand::rngs::StdRng>>;
}
