use crate::audio::audio_source;
use futures::stream::{FuturesOrdered, StreamExt};
use log::error;
use serenity::{
   client::Context,
   model::{
      id::ChannelId, id::GuildId, id::UserId, interactions::application_command::ApplicationCommandInteraction,
      user::User,
   },
   prelude::Mutex,
};
use songbird::{
   create_player,
   error::JoinError,
   input::{error::Error, Input},
   Call, Songbird,
};
use std::io::ErrorKind;
use std::sync::Arc;
use tokio::sync::MutexGuard;

pub async fn get_manager(ctx: &Context) -> Arc<Songbird> {
   songbird::get(ctx)
      .await
      .expect("Songbird voice client should have been placed during initialization")
}

pub struct ConnectionData {
   pub guild: GuildId,
   pub channel: ChannelId,
}

pub struct CallResult {
   pub user_message: String,
   pub underlying_error: Option<String>,
}

impl CallResult {
   pub fn success<T: ToString>(user_message: T) -> Self {
      Self {
         user_message: user_message.to_string(),
         underlying_error: None,
      }
   }

   pub fn failure<T: ToString, U: ToString>(user_message: T, underlying_error: U) -> Self {
      Self {
         user_message: user_message.to_string(),
         underlying_error: Some(underlying_error.to_string()),
      }
   }
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

async fn play_source(mut call: MutexGuard<'_, Call>, source: Input, volume: f32) {
   let (mut track, _) = create_player(source);
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
