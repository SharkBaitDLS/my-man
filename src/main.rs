mod audio;
mod chat;
mod event;
mod util;

use audio::playback;
use log::error;
use serenity::{client::bridge::gateway::GatewayIntents, client::Client, framework::StandardFramework};
use songbird::SerenityInit;
use std::env;

#[tokio::main]
async fn main() {
   env_logger::init();

   let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
   let mut client = Client::builder(token)
      .event_handler(event::listener::SoundboardListener)
      .framework(StandardFramework::new())
      .intents(
         GatewayIntents::GUILDS
            | GatewayIntents::GUILD_MEMBERS
            | GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::GUILD_VOICE_STATES
            | GatewayIntents::DIRECT_MESSAGES,
      )
      .register_songbird()
      .await
      .expect("Err creating client");

   if let Err(why) = client.start().await {
      error!("Client ended: {:?}", why)
   };
}
