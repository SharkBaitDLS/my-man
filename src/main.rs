mod actions;
mod audio;
mod call_result;
mod chat;
mod commands;
mod event;
mod guilds;
mod http;
mod role;

use log::error;
use rocket::routes;
use serenity::{
   client::{bridge::gateway::GatewayIntents, Cache, Client},
   framework::StandardFramework,
   http::Http,
};
use songbird::{SerenityInit, Songbird, SongbirdKey};
use std::{env, sync::Arc};

pub struct WebContext {
   pub cache: Arc<Cache>,
   pub http: Arc<Http>,
   pub songbird: Arc<Songbird>,
}

#[rocket::main]
async fn main() {
   env_logger::init();

   let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
   let application_id: u64 = env::var("APPLICATION_ID")
      .expect("Expected an app id in the environment")
      .parse()
      .expect("A valid numerical ID");
   env::var("WEB_URI").expect("Expected a web URI in the environment");

   let mut client = Client::builder(token)
      .application_id(application_id)
      .event_handler(event::listener::SoundboardListener)
      .framework(StandardFramework::new())
      .intents(GatewayIntents::GUILDS | GatewayIntents::GUILD_VOICE_STATES)
      .register_songbird()
      .await
      .expect("Err creating client");

   let rocket = rocket::build().mount("/", routes![http::play]).manage(WebContext {
      cache: client.cache_and_http.cache.clone(),
      http: client.cache_and_http.http.clone(),
      songbird: client
         .data
         .read()
         .await
         .get::<SongbirdKey>()
         .cloned()
         .expect("Songbird should be registered!"),
   });

   tokio::spawn(async move {
      if let Err(err) = client.start().await {
         error!("Client ended: {:?}", err)
      }
   });
   if let Err(err) = rocket.launch().await {
      error!("Webserver ended: {:?}", err)
   }
}
