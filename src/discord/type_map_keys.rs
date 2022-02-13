//! Collection of Serenity TypeMapKeys
use serenity::prelude::TypeMapKey;

pub struct GuildId;

impl TypeMapKey for GuildId {
    type Value = serenity::model::prelude::GuildId;
}

pub struct RedisConnection;
impl TypeMapKey for RedisConnection {
    type Value = redis::aio::Connection;
}
