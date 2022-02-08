use futures::{stream::FuturesOrdered, StreamExt};
use log::error;
use serenity::{
   client::Cache,
   http::Http,
   model::guild::{Guild, GuildInfo},
};

pub async fn get_bot_guild_infos(cache: &Cache, http: &Http) -> Vec<GuildInfo> {
   cache.current_user().await.guilds(http).await.unwrap_or_else(|err| {
      error!("Error retrieving this bot's guilds: {}", &err);
      Vec::new()
   })
}

pub async fn get_bot_guilds_cached(cache: &Cache, http: &Http) -> Vec<Guild> {
   get_bot_guild_infos(cache, http)
      .await
      .into_iter()
      .map(|info| info.id.to_guild_cached(cache))
      .collect::<FuturesOrdered<_>>()
      .filter_map(|guild| async { guild })
      .collect::<Vec<_>>()
      .await
}
