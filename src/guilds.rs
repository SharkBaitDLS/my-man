use log::error;
use serenity::{
   client::Cache,
   http::{CacheHttp, Http},
   model::{
      guild::{Guild, GuildInfo},
      id::GuildId,
   },
};

pub fn get_guild<T: AsRef<Cache>, G: Into<GuildId>>(cache: T, id: G) -> Option<Guild> {
   cache.as_ref().guild(id).map(|guild| guild.to_owned())
}

pub async fn get_bot_guild_infos<T: AsRef<Http>>(http: T) -> Vec<GuildInfo> {
   // We don't send any pagination data because My Man is (currently) not in more than 100 guilds
   http.as_ref().get_guilds(None, None).await.unwrap_or_else(|err| {
      error!("Error retrieving this bot's guilds: {}", &err);
      Vec::new()
   })
}

pub async fn get_bot_guilds_cached<T: CacheHttp + AsRef<Cache>>(cache_http: &T) -> Vec<Guild> {
   get_bot_guild_infos(cache_http.http())
      .await
      .into_iter()
      .filter_map(|info| get_guild(cache_http, info))
      .map(|cached| cached.to_owned())
      .collect::<Vec<_>>()
}
