use crate::database::StarChart;

use super::{ClientComponents, Config, State};
use miette::{IntoDiagnostic, Result, WrapErr};
use nebula::Leak;
use supernova::cloned;
use thiserror::Error;
use tokio::time::Instant;
use twilight_cache_inmemory::InMemoryCacheBuilder as CacheBuilder;
use twilight_gateway::{
	cluster::{ClusterBuilder, Events},
	Intents,
};
use twilight_http::client::ClientBuilder as HttpBuilder;
use twilight_standby::Standby;

#[derive(Debug, Error, Clone, Copy)]
pub enum StateBuilderError {
	#[error("intents not set")]
	Intents,
	#[error("config not set")]
	Config,
	#[error("cluster not built")]
	Cluster,
	#[error("database url not set")]
	Database
}

#[derive(Debug, Default)]
pub struct StateBuilder {
	cluster: Option<ClusterBuilder>,
	cache: Option<CacheBuilder>,
	http: Option<HttpBuilder>,
	intents: Option<Intents>,
	config: Option<Config>,
	database_url: Option<String>,
}

impl StateBuilder {
	#[must_use]
	pub const fn new() -> Self {
		Self {
			cluster: None,
			cache: None,
			http: None,
			intents: None,
			config: None,
			database_url: None,
		}
	}

	pub const fn config(mut self, config: Config) -> Result<Self> {
		self.config = Some(config);

		Ok(self)
	}

	pub const fn intents(mut self, intents: Intents) -> Result<Self> {
		self.intents = Some(intents);

		Ok(self)
	}

	pub fn database_url(mut self, url: impl Into<String>) -> Result<Self> {
		self.database_url = Some(url.into());

		Ok(self)
	}

	pub fn cluster_builder<F>(mut self, cluster_fn: F) -> Result<Self>
	where
		F: FnOnce(ClusterBuilder) -> ClusterBuilder,
	{
		let intents = self
			.intents
			.ok_or(StateBuilderError::Intents)
			.into_diagnostic()
			.context("need intents to build cluster")?;
		let token = Config::token()?;

		let cluster = cluster_fn((token, intents).into());

		self.cluster = Some(cluster);

		Ok(self)
	}

	pub fn cache_builder<F>(mut self, cache_fn: F) -> Result<Self>
	where
		F: FnOnce(CacheBuilder) -> CacheBuilder,
	{
		let built = cache_fn(CacheBuilder::default());

		self.cache = Some(built);

		Ok(self)
	}

	pub fn http_builder<F>(mut self, http_fn: F) -> Result<Self>
	where
		F: FnOnce(HttpBuilder) -> HttpBuilder,
	{
		let token = Config::token()?;
		let http_builder = self.http.map_or_else(
			move || HttpBuilder::new().token(token.to_owned()),
			|builder| builder,
		);
		let http = http_fn(http_builder);

		self.http = Some(http);

		Ok(self)
	}

	pub async fn build(self) -> Result<(State, Events)> {
		let config = self.config.unwrap_or_default();
		let token = Config::token()?.to_owned();
		let http_builder = self
			.http
			.unwrap_or_else(cloned!((token) => move || HttpBuilder::new().token(token)));
		let cluster_builder: ClusterBuilder = self
			.cluster
			.ok_or(StateBuilderError::Cluster)
			.into_diagnostic()
			.context("Need cluster to build state")?;
		let cache_builder = self.cache.unwrap_or_default();

		let http = http_builder.token(token).build();
		let cache = cache_builder.build();
		let (cluster, events) = cluster_builder
			.http_client(http.clone())
			.build()
			.await
			.into_diagnostic()?;
		let standby = Standby::new();

		let database = {
			let database_url: String = self.database_url.ok_or(StateBuilderError::Database).into_diagnostic()?;

			StarChart::new(&database_url).await.into_diagnostic()?
		};

		let components = unsafe {
			ClientComponents {
				cache,
				cluster,
				standby,
				http,
				runtime: Instant::now(),
				config,
				database,
			}
			.leak()
		};

		Ok((State(components), events))
	}
}
