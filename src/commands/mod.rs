use config::{CommandConfig, CommandOption};
use log::error;
use serenity::{
   client::Context,
   model::interactions::application_command::{ApplicationCommand, ApplicationCommandOptionType},
};

mod config;

pub async fn create_or_update(ctx: &Context) {
   let commands: Vec<CommandConfig> = vec![
      CommandConfig {
         name: "help",
         description: "Display help information",
         ..Default::default()
      },
      CommandConfig {
         name: "list",
         description: "List available sound files",
         ..Default::default()
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
         ..Default::default()
      },
      CommandConfig {
         name: "stop",
         description: "Stop the bot audio playback",
         ..Default::default()
      },
   ];

   if let Ok(current_commands) = ApplicationCommand::get_global_application_commands(ctx).await {
      for config in commands {
         match current_commands.iter().find(|command| command.name == config.name) {
            Some(command) if !config.is_equivalent(command) => (),
            _ => config.register_command(ctx).await,
         }
      }
   } else {
      error!("Could not load current commands from Discord, no changes will be made");
   }
}
