mod audio;
mod chat;
mod event;
mod util;

use audio::playback;
use serenity::{client::Client, framework::StandardFramework};
use std::{env, sync::Arc};

fn main() {
   env_logger::init();
   let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
   let mut client = Client::new(token, event::Listener).expect("Err creating client");

   {
      let mut data = client.data.write();
      data.insert::<playback::VoiceManager>(Arc::clone(&client.voice_manager));
   }

   client.with_framework(StandardFramework::new().configure(|c| c));

   let _ = client.start().map_err(|why| println!("Client ended: {:?}", why));
}
