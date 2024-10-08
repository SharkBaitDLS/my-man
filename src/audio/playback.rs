use crate::{
   audio::{audio_source, connection_data::ConnectionData},
   call_result::CallResult,
};
use reqwest::Client;
use serenity::{
   client::Context,
   model::{
      id::{ChannelId, GuildId, UserId},
      user::User,
   },
   prelude::Mutex,
};
use songbird::{
   error::{JoinError, JoinResult},
   input::{Input, YoutubeDl},
   tracks::Track,
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
   let track = Track::new(source).volume(volume);
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

async fn join_connection_with_manager(
   manager: Arc<Songbird>, connect_to: ConnectionData,
) -> Result<Arc<Mutex<Call>>, JoinError> {
   let call = manager.get_or_insert(connect_to.guild);
   let current_channel_id = { call.lock().await.current_channel() };

   if let Some(channel_id) = current_channel_id {
      if channel_id == connect_to.channel.into() {
         return Ok(call);
      }
   }
   match manager.join(connect_to.guild, connect_to.channel).await {
      JoinResult::Ok(call) => Ok(call),
      JoinResult::Err(err) => Err(err),
   }
}

pub async fn join_connection(ctx: &Context, connect_to: ConnectionData) -> Result<Arc<Mutex<Call>>, JoinError> {
   let manager = get_manager(ctx).await;

   join_connection_with_manager(manager, connect_to).await
}

async fn join_connection_with_manager_and_play(
   manager: Arc<Songbird>, connect_to: ConnectionData, source: Input, volume: f32,
) -> Result<(), JoinError> {
   match join_connection_with_manager(manager, connect_to).await {
      Ok(call) => {
         play_source(call.lock().await, source, volume).await;
         Ok(())
      }
      Err(err) => Err(err),
   }
}

pub async fn join_connection_and_play(
   ctx: &Context, connect_to: ConnectionData, source: Input, volume: f32,
) -> Result<(), JoinError> {
   join_connection_with_manager_and_play(get_manager(ctx).await, connect_to, source, volume).await
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

pub async fn play_youtube(ctx: &Context, client: Client, url: &str, connect_to: ConnectionData) -> CallResult {
   if !url.starts_with("http") {
      return CallResult::success(format!("{url} is not a valid URL"));
   }

   match join_connection_and_play(ctx, connect_to, YoutubeDl::new(client, url.to_owned()).into(), 1.0).await {
      Ok(_) => CallResult::success(format!("Playing {url}")),
      Err(err) => CallResult::failure("Failed to load youtube content", err),
   }
}

pub async fn play_file_with_manager(manager: Arc<Songbird>, name: &str, connect_to: ConnectionData) -> CallResult {
   match audio_source::file(name, &connect_to.guild).await {
      Ok(source) => match join_connection_with_manager_and_play(manager, connect_to, source, 1.0).await {
         Ok(_) => CallResult::success(format!("Playing {name}")),
         Err(err) => CallResult::failure(format!("Failed to load file for {name}"), err),
      },
      Err(err) if err.kind() == ErrorKind::NotFound => CallResult::success(format!("Audio file not found for {name}")),
      Err(err) => CallResult::failure(format!("Failed to load file for {name}"), err),
   }
}

pub async fn play_file(ctx: &Context, name: &str, connect_to: ConnectionData) -> CallResult {
   play_file_with_manager(get_manager(ctx).await, name, connect_to).await
}
