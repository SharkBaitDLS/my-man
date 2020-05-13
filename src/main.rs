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
      gateway::Activity,
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
use std::{collections::BinaryHeap, env, fs::File, fs::read_dir, io::ErrorKind, path::Path, sync::Arc};

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
   let file_names: BinaryHeap<String> = read_dir(file_dir).unwrap()
      .map(|path| String::from(path.unwrap().path().file_stem().unwrap().to_str().unwrap()))
      .collect();
   let list_message = file_names.into_sorted_vec().into_iter().fold(
      String::from("Type any of the following into the chat to play the sound:\n```\n"),
      |accum, path| accum + "?" + &path + "\n");
   return msg.content(list_message + "```");
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
         let safe_audio: LockedAudio = handler.play_returning(source);
         {
            let mut audio = safe_audio.lock();
            audio.volume(volume);
         }
      },
      None => error!("Could not load audio handler for playback.") 
   };
}

struct ConnectionData {
   guild: GuildId,
   channel: ChannelId
}

fn get_connection_data_from_message(ctx: &Context, msg: &Message) -> Option<ConnectionData> {
   let possible_guilds = match msg.guild(&ctx.cache) {
      Some(guild) => vec![guild],
      None => ctx.cache.read().user.guilds(&ctx.http).unwrap_or_else(|err| {
         error!("Error retrieving this bot's guilds: {}", &err);
         return Vec::new();
      }).into_iter().filter_map(|info| info.id.to_guild_cached(&ctx.cache)).collect()
   };

   return possible_guilds.into_iter()
      .find_map(|guild| match guild.read().voice_states.get(&msg.author.id).and_then(|state| state.channel_id) {
         Some(channel_id) => Some(ConnectionData { guild: guild.read().id, channel: channel_id }),
         None => None
      });
}

macro_rules! get_connection_data_or_return {
   ($ctx:ident, $msg:ident) => {
      match get_connection_data_from_message($ctx, $msg) {
         Some(data) => data,
         None => {
            log_on_error($msg.author.direct_message($ctx, |m| m.content("You are not in a voice channel!")));
            return;
         }
      };
   };
}

fn stop(ctx: &Context, msg: &Message) {
   let connect_to = get_connection_data_or_return!(ctx, msg);

   let manager_lock = ctx.data.read().get::<VoiceManager>().cloned().expect("Expected VoiceManager in data map.");
   let mut manager = manager_lock.lock();

   match manager.get_mut(connect_to.guild) {
      Some(handler) => handler.stop(),
      None => warn!("Could not load audio handler to stop.") 
   };
}

fn join_message_and_play(ctx: &Context, msg: &Message, source: Box<dyn voice::AudioSource>, volume: f32) {
   let connect_to = get_connection_data_or_return!(ctx, msg);
   join_and_play(ctx, connect_to.guild, connect_to.channel, source, volume);
}

fn join_message(ctx: &Context, msg: &Message) {
   let connect_to = get_connection_data_or_return!(ctx, msg);
   let manager_lock = ctx.data.read().get::<VoiceManager>().cloned().expect("Expected VoiceManager in data map.");
   let mut manager = manager_lock.lock();

   match manager.join(connect_to.guild, connect_to.channel) {
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
   fn ready(&self, ctx: Context, ready: Ready) {
      info!("{} is connected!", ready.user.name);
      ctx.set_activity(Activity::playing("Type ?help in chat"));
   }

   fn voice_state_update(&self, ctx: Context, guild_id: Option<GuildId>, old: Option<VoiceState>, new: VoiceState) {
      match new.channel_id {
         Some(channel_id) if old
            .and_then(|old_state| old_state.channel_id)
            .and_then(|old_channel_id| {
               let channel_changed = old_channel_id != channel_id;
               let changed_to_not_afk = channel_changed && guild_id.unwrap().to_guild_cached(&ctx.cache)
                  .and_then(|guild| guild.read().afk_channel_id)
                  .map_or(false, |afk_id| afk_id != channel_id);
               Option::from(changed_to_not_afk)
            })
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
         Some(_) => (),
         None => {
            match guild_id.and_then(|id| id.to_guild_cached(&ctx.cache)).map(|guild| guild.read().voice_states.len()) {
               Some(length) => {
                  // the bot is the only one left in voice
                  if length == 1 {
                     let manager_lock = ctx.data.read().get::<VoiceManager>().cloned()
                        .expect("Expected VoiceManager in data map.");
                     let mut manager = manager_lock.lock();
                     manager.leave(guild_id.unwrap());
                  }
               },
               None => ()
            }
         }
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
