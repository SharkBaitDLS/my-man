use crate::audio::{audio_source, connection_data::ConnectionData, playback};
use log::{error, warn};
use serenity::{
   client::Context,
   model::{
      id::{ChannelId, GuildId, UserId},
      voice::VoiceState,
   },
};
use std::collections::hash_map::{HashMap, Values};

fn is_afk_channel(ctx: &Context, guild_id: GuildId, channel_id: ChannelId) -> bool {
   guild_id
      .to_guild_cached(&ctx.cache)
      .and_then(|guild| guild.afk_channel_id)
      .map_or(false, |afk_id| afk_id == channel_id)
}

fn all_afk_states(ctx: &Context, guild_id: GuildId, states: Values<'_, UserId, VoiceState>) -> bool {
   let current_user_id = ctx.cache.current_user().id;
   states
      .filter(|state| state.user_id != current_user_id)
      .all(|state| state.channel_id.map_or(true, |id| is_afk_channel(ctx, guild_id, id)))
}

fn only_user_in_channel(ctx: &Context, states: &HashMap<UserId, VoiceState>) -> bool {
   let my_channel_id = states
      .get(&ctx.cache.current_user().id)
      .and_then(|user| user.channel_id);

   1 == states
      .values()
      .filter(|state| state.channel_id == my_channel_id)
      .count()
}

pub fn moved_to_non_afk(ctx: &Context, guild_id: GuildId, channel_id: ChannelId, old_id: Option<ChannelId>) -> bool {
   let moved_or_joined = old_id
      .map(|old_channel_id| old_channel_id != channel_id)
      .unwrap_or(true);

   moved_or_joined && !is_afk_channel(ctx, guild_id, channel_id)
}

pub async fn move_if_last_user(ctx: Context, guild_id: Option<GuildId>) {
   let current_user_id = ctx.cache.current_user().id;
   match guild_id
      .and_then(|id| id.to_guild_cached(&ctx.cache))
      .map(|guild| guild.voice_states)
   {
      // if the bot is the only one left in voice, disconnect from voice
      Some(states) if states.len() == 1 || all_afk_states(&ctx, guild_id.unwrap(), states.values()) => {
         let manager = playback::get_manager(&ctx).await;
         let _ = manager.leave(guild_id.unwrap()).await.map_err(|err| error!("{}", err));
      }
      // if the bot is the only one left in its channel, and others are active in the server, join them
      Some(states) if states.len() > 1 && only_user_in_channel(&ctx, &states) => {
         let first_active_channel = states
            .values()
            .filter(|state| state.user_id != current_user_id)
            .find_map(|state| {
               state
                  .channel_id
                  .filter(|channel_id| !is_afk_channel(&ctx, guild_id.unwrap(), *channel_id))
            });

         if let Some(channel_id) = first_active_channel {
            let connection = ConnectionData {
               guild: guild_id.unwrap(),
               channel: channel_id,
            };
            if let Ok(source) = audio_source::file("myman", &guild_id.unwrap()).await {
               if let Err(err) = playback::join_connection_and_play(&ctx, connection, source, 1.0).await {
                  error!("Failed to join another active channel: {}", err);
               }
            } else if let Err(err) = playback::join_connection(&ctx, connection).await {
               error!("Failed to join another active channel: {}", err);
            }
         } else {
            warn!("No channel found to join, but the number of states indicated there should be");
         }
      }
      _ => (),
   }
}
