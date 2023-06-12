use std::{env, path::PathBuf};

use log::{error, info};
use serenity::{
   client::{Context, EventHandler},
   model::{
      application::interaction::{Interaction, InteractionResponseType, MessageFlags},
      gateway::{Activity, Ready},
      guild::Guild,
      voice::VoiceState,
   },
   utils::Colour,
};

use crate::{actions, audio::playback, call_result, chat, commands, event::util, role};

pub struct SoundboardListener;

static HELP_MSG: &str = "You can type any of the following commands:
```
/list    - Returns a list of available sound files.
/play    - Plays the specified sound from the list.
/youtube - Plays the youtube link specified.
/stop    - Stops the currently playing sound(s).
/summon  - Summon the bot to your current voice channel.
```";

#[async_trait::async_trait]
impl EventHandler for SoundboardListener {
   async fn ready(&self, ctx: Context, ready: Ready) {
      info!("{} is connected!", ready.user.name);
      ctx.set_activity(Activity::listening("commands: /help")).await;
      commands::create_or_update(&ctx).await;
   }

   // Fired the first time the API sends data for a guild, even if it's not actually being created.
   // This should result in this event firing when the bot joins a new guild, or on bot startup.
   async fn guild_create(&self, ctx: Context, guild: Guild, _is_new: bool) {
      let file_dir = env::var("AUDIO_FILE_DIR").expect("Audio file directory must be in the environment!");
      let path: PathBuf = [file_dir, guild.id.as_u64().to_string()].iter().collect();

      match std::fs::create_dir_all(&path) {
         Ok(_) => role::create_admin_role(&ctx, &guild.id, path).await,
         Err(err) => error!("Could not generate clip directory for {}: {:?}", guild.id, err),
      }
   }

   async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
      match new.channel_id {
         Some(channel_id)
            if util::moved_to_non_afk(&ctx, new.guild_id.unwrap(), channel_id, old.and_then(|o| o.channel_id)) =>
         {
            let msg = call_result::log_error_if_any(
               playback::play_entrance(ctx, new.guild_id.unwrap(), channel_id, new.user_id).await,
            )
            .user_message;
            info!("{}", msg);
         }
         _ => util::move_if_last_user(ctx, new.guild_id).await,
      }
   }

   async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
      if let Interaction::ApplicationCommand(command) = interaction {
         // create an initial placeholder result that shows the bot as "thinking"
         let create_response = command
            .create_interaction_response(&ctx, |response| {
               response
                  .kind(InteractionResponseType::DeferredChannelMessageWithSource)
                  .interaction_response_data(|message| {
                     // Ephemeral means that only the user who issued the command sees the response
                     // and can dismiss it at their leisure
                     message.flags(MessageFlags::EPHEMERAL)
                  })
            })
            .await;

         if let Err(msg) = create_response {
            error!("Could not respond to command: {:?}", msg);
            return;
         }

         let result = match command.data.name.as_str() {
            "play" => actions::play(&ctx, &command).await,
            "youtube" => actions::youtube(&ctx, &command).await,
            "help" => HELP_MSG.to_string(),
            "list" => chat::list(&ctx, command.guild_id, &command.user).await,
            "stop" => actions::stop(&ctx, &command).await,
            "summon" => actions::summon(&ctx, &command).await,
            _ => "Unrecognized command!".to_string(),
         };

         // update the response with the actual result of the action
         let edit_response = command
            .edit_original_interaction_response(&ctx, |response| {
               response.embed(|embed| {
                  embed
                     .colour(Colour::FABLED_PINK)
                     .title(format!("/{} result", command.data.name))
                     .description(result)
               })
            })
            .await;
         if let Err(msg) = edit_response {
            error!("Could not respond to command: {:?}", msg);
         }
      }
   }
}
