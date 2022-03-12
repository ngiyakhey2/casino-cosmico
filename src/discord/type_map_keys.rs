//! Collection of Serenity TypeMapKeys
use crate::tito::client::Client;
use serenity::prelude::TypeMapKey;

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
