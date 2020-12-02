mod audio;
mod chat;
mod event;
mod util;

use audio::playback;
use log::error;
use serenity::{client::bridge::gateway::GatewayIntents, client::Client, framework::StandardFramework};
use std::{env, sync::Arc};

#[tokio::main]
async fn main() {
   env_logger::init();

   let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
   let mut client = Client::builder(token)
      .event_handler(event::listener::SoundboardListener)
      .framework(StandardFramework::new())
      .intents(GatewayIntents::DIRECT_MESSAGES | GatewayIntents::GUILD_MESSAGES | GatewayIntents::GUILD_VOICE_STATES)
      .await
      .expect("Err creating client");

   {
      let mut data = client.data.write().await;
      data.insert::<playback::VoiceManager>(Arc::clone(&client.voice_manager));
   }

   if let Err(why) = client.start().await {
      error!("Client ended: {:?}", why)
   };
}
