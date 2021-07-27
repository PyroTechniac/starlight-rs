use super::State;
use crate::lib::Config;
use anyhow::{Context, Result};
use std::sync::Arc;
use twilight_cache_inmemory::InMemoryCacheBuilder as CacheBuilder;
use twilight_gateway::{
    cluster::{ClusterBuilder, Events},
    Intents,
};
use twilight_http::client::ClientBuilder as HttpBuilder;
use twilight_standby::Standby;

#[derive(Debug, Default)]
pub struct StateBuilder {
    cluster: Option<ClusterBuilder>,
    cache: Option<CacheBuilder>,
    http: Option<HttpBuilder>,
    intents: Option<Intents>,
    config: Option<Config>,
}

impl StateBuilder {
    pub const fn new() -> Self {
        Self {
            cluster: None,
            cache: None,
            http: None,
            intents: None,
            config: None,
        }
    }

    pub fn config(mut self, config: Config) -> Self {
        self.config = Some(config);

        self
    }

    pub const fn intents(mut self, intents: Intents) -> Self {
        self.intents = Some(intents);

        self
    }

    pub fn cluster_builder<F>(mut self, cluster_fn: F) -> Self
    where
        F: FnOnce(ClusterBuilder) -> ClusterBuilder,
    {
        let intents = self
            .intents
            .context("need intents to build cluster")
            .unwrap();
        let token = self
            .config
            .clone()
            .context("need config to build cluster")
            .unwrap()
            .token;

        let cluster = cluster_fn((token, intents).into());

        self.cluster = Some(cluster);

        self
    }

    pub fn cache_builder<F>(mut self, cache_fn: F) -> Self
    where
        F: FnOnce(CacheBuilder) -> CacheBuilder,
    {
        let built = cache_fn(CacheBuilder::default());

        self.cache = Some(built);

        self
    }

    pub fn http_builder<F>(mut self, http_fn: F) -> Self
    where
        F: FnOnce(HttpBuilder) -> HttpBuilder,
    {
        let token = self
            .config
            .clone()
            .context("need config to build http")
            .unwrap()
            .token;
        let http_builder = self
            .http
            .map_or_else(move || HttpBuilder::new().token(token), |builder| builder);
        let http = http_fn(http_builder);

        self.http = Some(http);

        self
    }

    pub async fn build(self) -> Result<(State, Events)> {
        let token = self.config.clone().unwrap_or_default().token;
        let http_builder = self.http.unwrap_or_default();
        let cluster_builder = self.cluster.context("Need cluster to build state").unwrap();
        let cache_builder = self.cache.unwrap_or_default();

        let http = http_builder.token(token).build();
        let cache = cache_builder.build();
        let cluster = cluster_builder.http_client(http.clone()).build().await?;
        let standby = Standby::new();

        Ok((
            State {
                cache: Arc::new(cache),
                cluster: Arc::new(cluster.0),
                http: Arc::new(http),
                standby: Arc::new(standby),
                config: self.config.unwrap_or_default(),
            },
            cluster.1,
        ))
    }
}
