use crate::{
   audio::{audio_source, connection_data::ConnectionData, playback},
   call_result,
};
use log::error;
use reqwest::Client;
use serenity::{
   client::Context,
   model::application::{CommandDataOptionValue, CommandInteraction},
};

pub async fn play(ctx: &Context, command: &CommandInteraction) -> String {
   if let Some(connection) = ConnectionData::try_from_command(ctx, command).await {
      let option = &command.data.options.first().expect("Expected name option").value;

      if let CommandDataOptionValue::String(name) = option {
         call_result::log_error_if_any(playback::play_file(ctx, name, connection).await).user_message
      } else {
         "Cannot parse file name".to_string()
      }
   } else {
      "You are not in a voice channel!".to_string()
   }
}

pub async fn stop(ctx: &Context, command: &CommandInteraction) -> String {
   if let Some(connection) = ConnectionData::try_from_command(ctx, command).await {
      call_result::log_error_if_any(playback::stop(ctx, connection).await).user_message
   } else {
      "You are not in a guild with the bot!".to_string()
   }
}

pub async fn summon(ctx: &Context, command: &CommandInteraction) -> String {
   let msg: String;
   if let Some(connection) = ConnectionData::try_from_command(ctx, command).await {
      if let Ok(source) = audio_source::file("myman", &connection.guild).await {
         if let Err(err) = playback::join_connection_and_play(ctx, connection, source, 1.0).await {
            msg = "Bot failed to join your channel".to_string();
            error!("Failed to join summon: {}", err);
         } else {
            msg = "Bot summoned".to_string();
         }
      } else if let Err(err) = playback::join_connection(ctx, connection).await {
         msg = "Bot failed to join your channel".to_string();
         error!("Failed to join summon: {}", err);
      } else {
         msg = "Bot summoned".to_string();
      }
      msg
   } else {
      "You are not in a voice channel!".to_string()
   }
}

pub async fn youtube(ctx: &Context, client: Client, command: &CommandInteraction) -> String {
   if let Some(connection) = ConnectionData::try_from_command(ctx, command).await {
      let option = &command.data.options.first().expect("Expected URL option").value;

      if let CommandDataOptionValue::String(url) = option {
         call_result::log_error_if_any(playback::play_youtube(ctx, client, url, connection).await).user_message
      } else {
         "Cannot parse YouTube URL".to_string()
      }
   } else {
      "You are not in a voice channel!".to_string()
   }
}
