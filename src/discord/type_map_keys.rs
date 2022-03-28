//! Collection of Serenity TypeMapKeys
use crate::tito::client::Client;
use bb8_redis::RedisConnectionManager;
use rand::Rng as Rand;
use serenity::{
    model::prelude::GuildId as SerenityGuildId,
    prelude::{RwLock, TypeMap, TypeMapKey},
};
use std::sync::Arc;

pub struct GuildId;

impl TypeMapKey for GuildId {
    type Value = SerenityGuildId;
}

impl GuildId {
    pub async fn get(data: &Arc<RwLock<TypeMap>>) -> SerenityGuildId {
        let data = data.read().await;
        data.get::<Self>()
            .expect("Expected GuildId in TypeMap")
            .clone()
    }
}

pub struct RedisPool;
impl TypeMapKey for RedisPool {
    type Value = bb8::Pool<RedisConnectionManager>;
}

impl RedisPool {
    pub async fn get(data: &Arc<RwLock<TypeMap>>) -> bb8::Pool<RedisConnectionManager> {
        let data = data.read().await;
        data.get::<Self>()
            .expect("Expected RedisPool in TypeMap")
            .clone()
    }
}

pub struct TitoClient;
impl TypeMapKey for TitoClient {
    type Value = Client;
}

impl TitoClient {
    pub async fn get(data: &Arc<RwLock<TypeMap>>) -> Client {
        let data = data.read().await;
        data.get::<Self>()
            .expect("Expected TitoClient in TypeMap")
            .clone()
    }
}

pub struct Rng;
impl TypeMapKey for Rng {
    type Value = Arc<RwLock<rand::rngs::StdRng>>;
}

impl Rng {
    pub async fn rand(data: &Arc<RwLock<TypeMap>>, max: usize) -> usize {
        let data = data.read().await;
        let rng_lock = data.get::<Rng>().expect("Expected Rng in TypeMap");

        let mut rng = rng_lock.write().await;
        rng.gen_range(0..max)
    }
}
