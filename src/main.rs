mod audio;
mod chat;
mod event;
mod util;

use audio::playback;
use futures::executor::block_on;
use serenity::{client::bridge::gateway::GatewayIntents, client::Client, framework::StandardFramework};
use std::{env, sync::Arc};

fn main() {
   env_logger::init();
   let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
   block_on(async {
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

      let _ = client.start().await.map_err(|why| println!("Client ended: {:?}", why));
   })
}
