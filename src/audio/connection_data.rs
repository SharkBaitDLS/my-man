use serenity::{
   client::{Cache, Context},
   model::{
      application::CommandInteraction,
      id::{ChannelId, GuildId},
      user::User,
   },
};

use crate::guilds;

pub struct ConnectionData {
   pub guild: GuildId,
   pub channel: ChannelId,
}

impl ConnectionData {
   pub async fn try_from_command(ctx: &Context, command: &CommandInteraction) -> Option<Self> {
      match command.guild_id {
         Some(guild_id) => Self::try_from_guild_user(&ctx.cache, guild_id, &command.user),
         None => Self::try_from_user(ctx, &command.user).await,
      }
   }

   pub fn try_from_guild_user(cache: &Cache, guild_id: GuildId, user: &User) -> Option<Self> {
      guild_id.to_guild_cached(cache).and_then(|guild| {
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
