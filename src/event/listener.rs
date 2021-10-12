use crate::{
   actions,
   audio::playback,
   call_result, chat,
   event::{
      command_config::{CommandConfig, CommandOption},
      util,
   },
};
use log::{error, info};
use serenity::{
   client::{Context, EventHandler},
   model::{
      gateway::{Activity, Ready},
      id::GuildId,
      interactions::{
         application_command::{ApplicationCommand, ApplicationCommandOptionType},
         Interaction, InteractionApplicationCommandCallbackDataFlags, InteractionResponseType,
      },
      voice::VoiceState,
   },
   utils::Colour,
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

#[async_trait::async_trait]
impl EventHandler for SoundboardListener {
   async fn ready(&self, ctx: Context, ready: Ready) {
      info!("{} is connected!", ready.user.name);
      ctx.set_activity(Activity::listening("commands: /help")).await;

      let commands: Vec<CommandConfig> = vec![
         CommandConfig {
            name: "help",
            description: "Display help information",
            options: Vec::new(),
         },
         CommandConfig {
            name: "list",
            description: "List available sound files",
            options: Vec::new(),
         },
         CommandConfig {
            name: "play",
            description: "Play a sound file from the available library",
            options: vec![CommandOption {
               name: "name",
               description: "the name of the sound file",
               kind: ApplicationCommandOptionType::String,
               required: true,
            }],
         },
         CommandConfig {
            name: "youtube",
            description: "Play audio from a youtube video",
            options: vec![CommandOption {
               name: "url",
               description: "the YouTube URL",
               kind: ApplicationCommandOptionType::String,
               required: true,
            }],
         },
         CommandConfig {
            name: "summon",
            description: "Summon the bot to your voice channel",
            options: Vec::new(),
         },
         CommandConfig {
            name: "stop",
            description: "Stop the bot audio playback",
            options: Vec::new(),
         },
      ];

      if let Ok(current_commands) = ApplicationCommand::get_global_application_commands(&ctx).await {
         for config in commands {
            match current_commands.iter().find(|command| command.name == config.name) {
               Some(command) if !config.is_equivalent(command) => (),
               _ => config.register_command(&ctx).await,
            }
         }
      } else {
         error!("Could not load current commands from Discord, no changes will be made");
      }
   }

   async fn voice_state_update(
      &self, ctx: Context, guild_id: Option<GuildId>, old: Option<VoiceState>, new: VoiceState,
   ) {
      match new.channel_id {
         Some(channel_id)
            if util::moved_to_non_afk(&ctx, guild_id.unwrap(), channel_id, old.and_then(|o| o.channel_id)) =>
         {
            let msg = call_result::log_error_if_any(
               playback::play_entrance(ctx, guild_id.unwrap(), channel_id, new.user_id).await,
            )
            .user_message;
            info!("{}", msg);
         }
         _ => util::move_if_last_user(ctx, guild_id).await,
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
                     message.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
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
            &_ => "Unrecognized command!".to_string(),
         };

         // update the response with the actual result of the action
         let edit_response = command
            .edit_original_interaction_response(&ctx, |response| {
               response.create_embed(|embed| {
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
