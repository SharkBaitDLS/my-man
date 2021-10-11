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

pub async fn get_connection_data_for_command(
   ctx: &Context, command: &ApplicationCommandInteraction,
) -> Option<ConnectionData> {
   match command.guild_id {
      Some(guild_id) => get_connection_data_for_guild(ctx, guild_id, &command.user).await,
      None => get_connection_data_for_user(ctx, &command.user).await,
   }
}

async fn get_connection_data_for_guild(ctx: &Context, guild_id: GuildId, user: &User) -> Option<ConnectionData> {
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

async fn get_connection_data_for_user(ctx: &Context, user: &User) -> Option<ConnectionData> {
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
