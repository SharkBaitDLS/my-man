use crate::audio::{audio_source, playback};
use crate::chat;
use crate::playback::get_connection_data_from_message;
use crate::util::log_on_error;
use futures::executor::block_on;
use log::{debug, error, info, warn};
use serenity::{
   client::Context,
   model::{channel::Message, id::ChannelId, id::GuildId, id::UserId, user::User, voice::VoiceState},
};
use std::collections::hash_map::{HashMap, Values};

fn is_afk_channel(ctx: &Context, guild_id: GuildId, channel_id: ChannelId) -> bool {
   block_on(async {
      guild_id
         .to_guild_cached(&ctx.cache)
         .await
         .and_then(|guild| guild.afk_channel_id)
         .map_or(false, |afk_id| afk_id == channel_id)
   })
}

fn all_afk_states(ctx: &Context, guild_id: GuildId, states: Values<'_, UserId, VoiceState>) -> bool {
   let current_user_id = block_on(ctx.cache.current_user()).id;
   states
      .filter(|state| state.user_id != current_user_id)
      .all(|state| state.channel_id.map_or(true, |id| is_afk_channel(ctx, guild_id, id)))
}

fn only_user_in_channel(ctx: &Context, states: &HashMap<UserId, VoiceState>) -> bool {
   let my_channel_id = states
      .get(&block_on(ctx.cache.current_user()).id)
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
   let current_user_id = ctx.cache.current_user().await.id;
   match guild_id
      .and_then(|id| block_on(id.to_guild_cached(&ctx.cache)))
      .map(|guild| guild.voice_states)
   {
      // if the bot is the only one left in voice, disconnect from voice
      Some(states) if states.len() == 1 || all_afk_states(&ctx, guild_id.unwrap(), states.values()) => {
         let manager = playback::get_manager(ctx).await;
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
            if let Some(source) =
               audio_source::file("myman", &guild_id.unwrap(), |file| info!("{} not found", file)).await
            {
               playback::join_and_play(ctx, guild_id.unwrap(), channel_id, source, 1.0).await
            } else {
               let manager = playback::get_manager(ctx).await;
               if let (_, Err(err)) = manager.join(guild_id.unwrap(), channel_id).await {
                  error!("Failed to join channel: {}", err);
               }
            }
         } else {
            warn!("No channel found to join, but the number of states indicated there should be");
         }
      }
      _ => (),
   }
}

pub async fn play_entrance(ctx: Context, guild_id: GuildId, channel_id: ChannelId, user_id: UserId) {
   match user_id.to_user(&ctx).await {
      Ok(user) => match user {
         User { bot: true, .. } => debug!("A bot joined a channel: {}", user.name),
         _ => {
            if let Some(source) =
               audio_source::file(&user.name, &guild_id, |file| info!("No user file found for {}", file)).await
            {
               playback::join_and_play(ctx, guild_id, channel_id, source, 1.0).await
            }
         }
      },
      Err(why) => error!("Could not get user name: {}", why.to_string()),
   }
}

pub async fn play_youtube(ctx: Context, msg: Message) {
   let url = msg.content.split_at(4).1;
   if !url.starts_with("http") {
      log_on_error(
         msg.author
            .direct_message(ctx, |m| m.content("You must provide a valid YouTube URL!")),
      )
      .await;
      return;
   };
   match audio_source::youtube(&url).await {
      Some(source) => playback::join_message_and_play(ctx, msg, source, 0.2).await,
      None => error!("Could not play youtube video at {}", url),
   }
}

pub fn get_file_name(msg: &Message) -> &str {
   msg.content.split_at(1).1
}

pub async fn play_file(ctx: Context, msg: Message) {
   let name = get_file_name(&msg);
   if let Some(connect_to) = get_connection_data_from_message(&ctx, &msg).await {
      if let Some(source) =
         audio_source::file_async_nf(name, &connect_to.guild, |name| chat::dm_not_found(&ctx, &msg, name)).await
      {
         playback::join_connection_and_play(ctx, connect_to, source, 1.0).await
      }
   }
}
