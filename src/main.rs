extern crate serenity;

use serenity::voice::LockedAudio;
use log::{debug, info, error, warn};
use serenity::{
   builder::CreateMessage,
   client::{Client, Context, EventHandler, bridge::voice::ClientVoiceManager},
   framework::StandardFramework,
   model::{
      channel::Message,
      channel::MessageType,
      gateway::Ready,
      id::ChannelId,
      id::GuildId,
      user::User,
      voice::VoiceState
   },
   prelude::Mutex,
   prelude::*,
   Result as SerenityResult,
   voice
};
use std::{env, fs::File, fs::read_dir, io::ErrorKind, path::Path, sync::Arc}; 

struct VoiceManager;

impl TypeMapKey for VoiceManager {
   type Value = Arc<Mutex<ClientVoiceManager>>;
}

fn log_on_error<T>(result: SerenityResult<T>) {
   if let Err(why) = result {
      error!("Failed discord call: {:?}", why);
   }
}

fn help<'a, 'b>(msg: &'a mut CreateMessage<'b>) -> &'a mut CreateMessage<'b> {
   return msg.content("You can type any of the following commands:
```
?list             - Returns a list of available sound files.
?soundFileName    - Plays the specified sound from the list.
?yt youtubeLink   - Plays the youtube link specified.
?stop             - Stops the sound that is currently playing.
?summon           - Summon the bot to your channel.
```")
}

fn list<'a, 'b>(msg: &'a mut CreateMessage<'b>) -> &'a mut CreateMessage<'b> {
   let file_dir = env::var("AUDIO_FILE_DIR").expect("Audio file directory must be in the environment!");
   let file_names = read_dir(file_dir).unwrap()
      .fold(String::from("Type any of the following into the chat to play the sound:\n```\n"),
         |accum, path| accum + "?" + path.unwrap().path().file_stem().unwrap().to_str().unwrap() + "\n");
   return msg.content(file_names + "```");
}

fn stop(ctx: &Context, msg: &Message) {
   let guild = match msg.guild(&ctx.cache) {
      Some(guild) => guild,
      None => {
         log_on_error(msg.author.direct_message(ctx, |m| m.content(
            "I don't know what guild to join when you DM me, please write in a chat channel.")));
         return;
      }
   };
   let guild_id = guild.read().id;

   let manager_lock = ctx.data.read().get::<VoiceManager>().cloned().expect("Expected VoiceManager in data map.");
   let mut manager = manager_lock.lock();

   match manager.get_mut(guild_id) {
      Some(handler) => handler.stop(),
      None => warn!("Could not load audio handler to stop.") 
   };
}

fn join_and_play(
   ctx: &Context,
   guild_id: GuildId,
   channel_id: ChannelId,
   source: Box<dyn voice::AudioSource>,
   volume: f32
) {
   let manager_lock = ctx.data.read().get::<VoiceManager>().cloned().expect("Expected VoiceManager in data map.");
   let mut manager = manager_lock.lock();

   match manager.join(guild_id, channel_id) {
      Some(handler) => {
         let safe_audio: LockedAudio = handler.play_only(source);
         {
            let mut audio = safe_audio.lock();
            audio.volume(volume);
         }
      },
      None => error!("Could not load audio handler for playback.") 
   };
}

fn join_message_and_play(ctx: &Context, msg: &Message, source: Box<dyn voice::AudioSource>, volume: f32) {
   let guild = match msg.guild(&ctx.cache) {
      Some(guild) => guild,
      None => {
         log_on_error(msg.author.direct_message(ctx, |m| m.content(
            "I don't know what guild to join when you DM me, please write in a chat channel.")));
         return;
      }
   };

   let guild_id = guild.read().id;
   let channel_id = guild.read().voice_states.get(&msg.author.id).and_then(|voice_state| voice_state.channel_id);

   let connect_to = match channel_id {
      Some(channel) => channel,
      None => {
         log_on_error(msg.author.direct_message(ctx, |m| m.content("You are not in a voice channel!")));
         return;
      }
   };

   join_and_play(ctx, guild_id, connect_to, source, volume);
}

fn join_message(ctx: &Context, msg: &Message) {
   let guild = match msg.guild(&ctx.cache) {
      Some(guild) => guild,
      None => {
         log_on_error(msg.author.direct_message(ctx, |m| m.content(
            "I don't know what guild to join when you DM me, please write in a chat channel.")));
         return;
      }
   };

   let guild_id = guild.read().id;
   let channel_id = guild.read().voice_states.get(&msg.author.id).and_then(|voice_state| voice_state.channel_id);

   let connect_to = match channel_id {
      Some(channel) => channel,
      None => {
         log_on_error(msg.author.direct_message(ctx, |m| m.content("You are not in a voice channel!")));
         return;
      }
   };

   let manager_lock = ctx.data.read().get::<VoiceManager>().cloned().expect("Expected VoiceManager in data map.");
   let mut manager = manager_lock.lock();

   match manager.join(guild_id, connect_to) {
      Some(_) => (),
      None => error!("Could not load audio handler for playback.") 
   };
}

fn get_file_source<F>(name: &String, not_found_handler: F) -> Option<Box<dyn voice::AudioSource>> where F: Fn(&String) {
   let file_dir = env::var("AUDIO_FILE_DIR").expect("Audio file directory must be in the environment!");
   let audio_file_path_str = file_dir + &name.to_lowercase() + ".mp3";
   let path = Path::new(&(audio_file_path_str));

   match File::open(&path) {
      Err(why) => {
         match why.kind() {
            ErrorKind::NotFound => not_found_handler(name),
            _ => error!("couldn't open {}: {}", audio_file_path_str, why.to_string())
         };
         return None;
      },
      Ok(file) => file
   };

   match voice::ffmpeg(path) {
      Ok(source) => Option::from(source),
      Err(why) => {
         error!("Err starting source: {:?}", why);
         None
      },
   }
}

fn get_youtube_source(url: String) -> Option<Box<dyn voice::AudioSource>> {
   match voice::ytdl(&url) {
      Ok(source) => Option::from(source),
      Err(why) => {
         error!("Err starting source: {:?}", why);
         None
      },
   }
}

fn dm_not_found(ctx: &Context, msg: &Message, name: &String) {
   log_on_error(msg.author.direct_message(ctx, |m| m.content(format!("Cannot find audio file for {}", name))));
}

struct EventListener;

impl EventHandler for EventListener {
   fn ready(&self, _ctx: Context, ready: Ready) {
      info!("{} is connected!", ready.user.name);
   }

   fn voice_state_update(&self, ctx: Context, guild_id: Option<GuildId>, old: Option<VoiceState>, new: VoiceState) {
      match new.channel_id {
         Some(channel_id) if old
            .and_then(|old_state| old_state.channel_id)
            .and_then(|old_channel_id| Option::from(old_channel_id != channel_id))
            .unwrap_or_else(|| true) =>
               match new.user_id.to_user(&ctx) {
                  Ok(user) => match user {
                     User { bot: true, .. } => debug!("A bot joined a channel: {}", user.name),
                     _ => match get_file_source(&user.name, |file| info!("No user sound file found for {}", file)) {
                        Some(source) => join_and_play(&ctx, guild_id.unwrap(), channel_id, source, 1.0),
                        None => error!("Could not play sound for voice state update.")
                     }
                  },
                  Err(why) => error!("Could not get user name: {}", why.to_string())
               },
         _ => ()
      }
   }

   fn message(&self, ctx: Context, msg: Message) {
      if let MessageType::Regular = msg.kind {
         if msg.content.starts_with("?") {
            if !msg.is_private() {
               log_on_error(msg.delete(&ctx));
            }
            match msg.content.as_ref() {
               "?list" => log_on_error(msg.author.direct_message(&ctx, list)),
               content if content.starts_with("?yt ") => {
                  let url = msg.content.split_at(4).1.to_string();
                  if !url.starts_with("http") {
                     log_on_error(msg.author.direct_message(&ctx, |m| m.content("Must provide a valid URL")));
                     return;
                  };
                  match get_youtube_source(url) {
                     Some(source) => join_message_and_play(&ctx, &msg, source, 0.2),
                     None => error!("Could not play youtube video.")
                  }
               },
               "?stop" => stop(&ctx, &msg),
               "?summon" => join_message(&ctx, &msg),
               "?help" => log_on_error(msg.author.direct_message(&ctx, help)),
               _ => match get_file_source(&msg.content.split_at(1).1.to_string(), |n| dm_not_found(&ctx, &msg, n)) {
                  Some(source) => join_message_and_play(&ctx, &msg, source, 1.0),
                  None => error!("Could not play sound for chat request.")
               }
            };
         }
      }
   }
}

fn main() {
   env_logger::init();
   let token = env::var("DISCORD_TOKEN")
      .expect("Expected a token in the environment");
   let mut client = Client::new(&token, EventListener).expect("Err creating client");

   {
      let mut data = client.data.write();
      data.insert::<VoiceManager>(Arc::clone(&client.voice_manager));
   }

   client.with_framework(StandardFramework::new().configure(|c| c));

   let _ = client.start().map_err(|why| println!("Client ended: {:?}", why));
}
