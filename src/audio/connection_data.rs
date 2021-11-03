use serenity::{
   client::Context,
   model::{
      id::{ChannelId, GuildId},
      interactions::application_command::ApplicationCommandInteraction,
      user::User,
   },
};

use crate::guilds;

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
      guilds::get_bot_guilds_cached(ctx).await.into_iter().find_map(|guild| {
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
