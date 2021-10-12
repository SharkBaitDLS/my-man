use crate::{
   audio::{audio_source, connection_data::ConnectionData},
   call_result::CallResult,
};
use serenity::{
   client::Context,
   model::{
      id::{ChannelId, GuildId, UserId},
      user::User,
   },
   prelude::Mutex,
};
use songbird::{
   error::JoinError,
   input::{error::Error, Input},
   Call, Songbird,
};
use std::{io::ErrorKind, sync::Arc};
use tokio::sync::MutexGuard;

pub async fn get_manager(ctx: &Context) -> Arc<Songbird> {
   songbird::get(ctx)
      .await
      .expect("Songbird voice client should have been placed during initialization")
}

async fn play_source(mut call: MutexGuard<'_, Call>, source: Input, volume: f32) {
   let (mut track, _) = songbird::create_player(source);
   track.set_volume(volume);
   call.play(track);
}

pub async fn stop(ctx: &Context, connect_to: ConnectionData) -> CallResult {
   let manager = get_manager(ctx).await;

   if let Some(call) = manager.get(connect_to.guild) {
      let mut locked = call.lock().await;
      if let Some(channel_id) = locked.current_channel() {
         if channel_id == connect_to.channel.into() {
            locked.stop();
            return CallResult::success("Playback stopped");
         }
      }
   };
   CallResult::failure(
      "Bot is not currently in your channel".to_string(),
      "Bot in a different channel than requestor",
   )
}

pub async fn join_connection(ctx: &Context, connect_to: ConnectionData) -> Result<Arc<Mutex<Call>>, JoinError> {
   let manager = get_manager(ctx).await;

   let call = manager.get_or_insert(connect_to.guild.into());
   let current_channel_id = { call.lock().await.current_channel() };

   if let Some(channel_id) = current_channel_id {
      if channel_id == connect_to.channel.into() {
         return Ok(call);
      }
   }
   match manager.join(connect_to.guild, connect_to.channel).await {
      (call, Ok(_)) => Ok(call),
      (_, Err(err)) => Err(err),
   }
}

pub async fn join_connection_and_play(
   ctx: &Context, connect_to: ConnectionData, source: Input, volume: f32,
) -> Result<(), JoinError> {
   match join_connection(ctx, connect_to).await {
      Ok(call) => {
         play_source(call.lock().await, source, volume).await;
         Ok(())
      }
      Err(err) => Err(err),
   }
}

pub async fn play_entrance(ctx: Context, guild_id: GuildId, channel_id: ChannelId, user_id: UserId) -> CallResult {
   match user_id.to_user(&ctx).await {
      Ok(user) => match user {
         User { bot: true, .. } => CallResult::success(format!("A bot joined a channel: {}", user.name)),
         _ => {
            play_file(
               &ctx,
               &user.name,
               ConnectionData {
                  guild: guild_id,
                  channel: channel_id,
               },
            )
            .await
         }
      },
      Err(err) => CallResult::failure("Could not get user name", err),
   }
}

pub async fn play_youtube(ctx: &Context, url: &str, connect_to: ConnectionData) -> CallResult {
   if !url.starts_with("http") {
      return CallResult::success(format!("{} is not a valid URL", url));
   }
   if let Ok(source) = songbird::ytdl(url).await {
      match join_connection_and_play(ctx, connect_to, source, 0.2).await {
         Ok(_) => CallResult::success(format!("Playing {}", url)),
         Err(err) => CallResult::failure("Failed to load youtube content", err),
      }
   } else {
      CallResult::success(format!("Youtube content not found for {}", url))
   }
}

pub async fn play_file(ctx: &Context, name: &str, connect_to: ConnectionData) -> CallResult {
   match audio_source::file(name, &connect_to.guild).await {
      Ok(source) => match join_connection_and_play(ctx, connect_to, source, 1.0).await {
         Ok(_) => CallResult::success(format!("Playing {}", name)),
         Err(err) => CallResult::failure(format!("Failed to load file for {}", name), err),
      },
      Err(Error::Io(err)) if err.kind() == ErrorKind::NotFound => {
         CallResult::success(format!("Audio file not found for {}", name))
      }
      Err(err) => CallResult::failure(format!("Failed to load file for {}", name), err),
   }
}
