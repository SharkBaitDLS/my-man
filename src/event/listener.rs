use crate::audio::audio_source;
use crate::call_result;
use crate::chat;
use crate::event::util::{move_if_last_user, moved_to_non_afk};
use crate::playback;
use async_trait::async_trait;
use log::{error, info};
use serenity::{
   client::{Context, EventHandler},
   model::{
      gateway::{Activity, Ready},
      id::GuildId,
      interactions::{
         application_command::{
            ApplicationCommand, ApplicationCommandInteractionDataOptionValue, ApplicationCommandOptionType,
         },
         Interaction, InteractionApplicationCommandCallbackDataFlags, InteractionResponseType,
      },
      voice::VoiceState,
   },
};

pub struct SoundboardListener;

static HELP_MSG: &str = "You can type any of the following commands:
```
/list    - Returns a list of available sound files.
/play    - Plays the specified sound from the list.
/yt      - Plays the youtube link specified.
/stop    - Stops the currently playing sound(s).
/summon  - Summon the bot to your current voice channel.
```";

#[async_trait]
impl EventHandler for SoundboardListener {
   async fn ready(&self, ctx: Context, ready: Ready) {
      info!("{} is connected!", ready.user.name);
      ctx.set_activity(Activity::listening("commands: /help")).await;

      let commands = ApplicationCommand::set_global_application_commands(&ctx, |commands| {
         commands
            .create_application_command(|command| command.name("help").description("Display help information"))
            .create_application_command(|command| command.name("list").description("List available sound files"))
            .create_application_command(|command| {
               command
                  .name("play")
                  .description("Play a sound file from the available library")
                  .create_option(|option| {
                     option
                        .name("name")
                        .description("the name of the sound file")
                        .kind(ApplicationCommandOptionType::String)
                        .required(true)
                  })
            })
            .create_application_command(|command| {
               command
                  .name("youtube")
                  .description("Play audio from a youtube video")
                  .create_option(|option| {
                     option
                        .name("url")
                        .description("the youtube URL")
                        .kind(ApplicationCommandOptionType::String)
                        .required(true)
                  })
            })
            .create_application_command(|command| {
               command
                  .name("summon")
                  .description("Summon the bot to your voice channel")
            })
            .create_application_command(|command| command.name("stop").description("Stop the bot audio playback"))
      })
      .await;

      if let Err(msg) = commands {
         error!("Could not register commands: {:?}", msg);
      }
   }

   async fn voice_state_update(
      &self, ctx: Context, guild_id: Option<GuildId>, old: Option<VoiceState>, new: VoiceState,
   ) {
      match new.channel_id {
         Some(channel_id) if moved_to_non_afk(&ctx, guild_id.unwrap(), channel_id, old.and_then(|o| o.channel_id)) => {
            let msg = call_result::log_error_if_any(
               playback::play_entrance(ctx, guild_id.unwrap(), channel_id, new.user_id).await,
            )
            .user_message;
            info!("{}", msg);
         }
         _ => move_if_last_user(ctx, guild_id).await,
      }
   }

   // TODO: break out logic into "actions" module
   async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
      if let Interaction::ApplicationCommand(command) = interaction {
         // create an initial placeholder result that shows the bot as "thinking"
         let create_response = command
            .create_interaction_response(&ctx, |response| {
               response
                  .kind(InteractionResponseType::DeferredChannelMessageWithSource)
                  .interaction_response_data(|message| {
                     message.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                  })
            })
            .await;

         if let Err(msg) = create_response {
            error!("Could not respond to command: {:?}", msg);
            return;
         }

         let result = match command.data.name.as_str() {
            "play" => {
               if let Some(connection) = playback::get_connection_data_for_command(&ctx, &command).await {
                  let option = command
                     .data
                     .options
                     .get(0)
                     .expect("Expected name option")
                     .resolved
                     .as_ref()
                     .expect("Expected a value to be passed");

                  if let ApplicationCommandInteractionDataOptionValue::String(name) = option {
                     call_result::log_error_if_any(playback::play_file(&ctx, name, connection).await).user_message
                  } else {
                     "Cannot parse file name".to_string()
                  }
               } else {
                  "You are not in a voice channel!".to_string()
               }
            }
            "youtube" => {
               if let Some(connection) = playback::get_connection_data_for_command(&ctx, &command).await {
                  let option = command
                     .data
                     .options
                     .get(0)
                     .expect("Expected URL option")
                     .resolved
                     .as_ref()
                     .expect("Expected a value to be passed");

                  if let ApplicationCommandInteractionDataOptionValue::String(url) = option {
                     call_result::log_error_if_any(playback::play_youtube(&ctx, url, connection).await).user_message
                  } else {
                     "Cannot parse YouTube URL".to_string()
                  }
               } else {
                  "You are not in a voice channel!".to_string()
               }
            }
            "help" => HELP_MSG.to_string(),
            "list" => chat::list(&ctx, command.guild_id, &command.user).await,
            "stop" => {
               if let Some(connection) = playback::get_connection_data_for_command(&ctx, &command).await {
                  call_result::log_error_if_any(playback::stop(&ctx, connection).await).user_message
               } else {
                  "You are not in a guild with the bot!".to_string()
               }
            }
            "summon" => {
               let msg: String;
               if let Some(connection) = playback::get_connection_data_for_command(&ctx, &command).await {
                  if let Ok(source) = audio_source::file("myman", &connection.guild).await {
                     if let Err(err) = playback::join_connection_and_play(&ctx, connection, source, 1.0).await {
                        msg = "Bot failed to join your channel".to_string();
                        error!("Failed to join summon: {}", err);
                     } else {
                        msg = "Bot summoned".to_string();
                     }
                  } else if let Err(err) = playback::join_connection(&ctx, connection).await {
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
            &_ => "Unrecognized command!".to_string(),
         };

         let edit_response = command
            .edit_original_interaction_response(&ctx, |response| response.content(result))
            .await;
         if let Err(msg) = edit_response {
            error!("Could not respond to command: {:?}", msg);
         }
      }
   }
}
