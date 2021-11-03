use futures::{stream::FuturesOrdered, StreamExt};
use log::error;
use serenity::{
   client::Context,
   model::guild::{Guild, GuildInfo},
};

pub async fn get_bot_guild_infos(ctx: &Context) -> Vec<GuildInfo> {
   ctx.cache
      .current_user()
      .await
      .guilds(&ctx.http)
      .await
      .unwrap_or_else(|err| {
         error!("Error retrieving this bot's guilds: {}", &err);
         Vec::new()
      })
}

pub async fn get_bot_guilds_cached(ctx: &Context) -> Vec<Guild> {
   get_bot_guild_infos(ctx)
      .await
      .into_iter()
      .map(|info| info.id.to_guild_cached(&ctx.cache))
      .collect::<FuturesOrdered<_>>()
      .filter_map(|guild| async { guild })
      .collect::<Vec<_>>()
      .await
}
