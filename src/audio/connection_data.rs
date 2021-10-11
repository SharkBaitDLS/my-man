use futures::{stream::FuturesOrdered, StreamExt};
use log::error;
use serenity::client::Context;
use serenity::model::{
   id::{ChannelId, GuildId},
   interactions::application_command::ApplicationCommandInteraction,
   user::User,
};

pub struct ConnectionData {
   pub guild: GuildId,
   pub channel: ChannelId,
}

impl ConnectionData {
   pub async fn try_from_command(ctx: &Context, command: &ApplicationCommandInteraction) -> Option<Self> {
      match command.guild_id {
         Some(guild_id) => Self::try_from_guild_user(ctx, guild_id, &command.user).await,
         None => Self::try_from_user(ctx, &command.user).await,
      }
   }

   async fn try_from_guild_user(ctx: &Context, guild_id: GuildId, user: &User) -> Option<Self> {
      guild_id.to_guild_cached(ctx).await.and_then(|guild| {
         guild
            .voice_states
            .get(&user.id)
            .and_then(|state| state.channel_id)
            .map(|channel_id| ConnectionData {
               guild: guild.id,
               channel: channel_id,
            })
      })
   }

   async fn try_from_user(ctx: &Context, user: &User) -> Option<Self> {
      let possible_guilds = ctx
         .cache
         .current_user()
         .await
         .guilds(&ctx.http)
         .await
         .unwrap_or_else(|err| {
            error!("Error retrieving this bot's guilds: {}", &err);
            Vec::new()
         })
         .into_iter()
         .map(|info| info.id.to_guild_cached(&ctx.cache))
         .collect::<FuturesOrdered<_>>()
         .filter_map(|guild| async { guild })
         .collect::<Vec<_>>()
         .await;

      possible_guilds.into_iter().find_map(|guild| {
         guild
            .voice_states
            .get(&user.id)
            .and_then(|state| state.channel_id)
            .map(|channel_id| ConnectionData {
               guild: guild.id,
               channel: channel_id,
            })
      })
   }
}
