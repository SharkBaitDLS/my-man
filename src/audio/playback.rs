use crate::util::log_on_error;
use log::{error, warn};
use serenity::voice::{Handler, LockedAudio};
use serenity::{
   client::{bridge::voice::ClientVoiceManager, Context},
   model::{channel::Message, id::ChannelId, id::GuildId},
   prelude::Mutex,
   prelude::*,
   voice,
};
use std::{sync::Arc, thread::sleep, time::Duration};

pub struct VoiceManager;

impl TypeMapKey for VoiceManager {
   type Value = Arc<Mutex<ClientVoiceManager>>;
}

fn get_manager_lock(ctx: Context) -> Arc<Mutex<ClientVoiceManager>> {
   return ctx
      .data
      .read()
      .get::<VoiceManager>()
      .cloned()
      .expect("Expected VoiceManager in data map");
}

struct ConnectionData {
   guild: GuildId,
   channel: ChannelId,
}

fn get_connection_data_from_message(ctx: &Context, msg: &Message) -> Option<ConnectionData> {
   let possible_guilds = match msg.guild(&ctx.cache) {
      Some(guild) => vec![guild],
      None => ctx
         .cache
         .read()
         .user
         .guilds(&ctx.http)
         .unwrap_or_else(|err| {
            error!("Error retrieving this bot's guilds: {}", &err);
            return Vec::new();
         })
         .into_iter()
         .filter_map(|info| info.id.to_guild_cached(&ctx.cache))
         .collect(),
   };

   return possible_guilds.into_iter().find_map(|guild| {
      match guild
         .read()
         .voice_states
         .get(&msg.author.id)
         .and_then(|state| state.channel_id)
      {
         Some(channel_id) => Some(ConnectionData {
            guild: guild.read().id,
            channel: channel_id,
         }),
         None => None,
      }
   });
}

macro_rules! get_connection_data_or_return {
   ($ctx:ident, $msg:ident) => {
      match get_connection_data_from_message(&$ctx, &$msg) {
         Some(data) => data,
         None => {
            log_on_error(
               $msg
                  .author
                  .direct_message($ctx, |m| m.content("You are not in a voice channel!")),
            );
            return;
         }
      };
   };
}

fn play_source(handler: &mut Handler, source: Box<dyn voice::AudioSource>, volume: f32) {
   let safe_audio: LockedAudio = handler.play_returning(source);
   {
      let mut audio = safe_audio.lock();
      audio.volume(volume);
   }
}

pub fn join_and_play(
   ctx: Context, guild_id: GuildId, channel_id: ChannelId, source: Box<dyn voice::AudioSource>, volume: f32,
) {
   let manager_lock = get_manager_lock(ctx);
   let mut manager = manager_lock.lock();

   match manager.get_mut(guild_id) {
      Some(handler) => {
         if handler.channel_id != Some(channel_id) {
            handler.join(channel_id);
            // the underlying HTTP request to Discord's API to switch channels
            // doesn't immediately take effect, so the call above returning doesn't actually
            // mean the switch has happened
            sleep(Duration::from_secs(1));
         }
         play_source(handler, source, volume);
      }
      None => match manager.join(guild_id, channel_id) {
         Some(handler) => {
            sleep(Duration::from_secs(1));
            play_source(handler, source, volume);
         }
         None => error!("Could not create audio handler for initial join"),
      },
   };
}

pub fn stop(ctx: Context, msg: Message) {
   let connect_to = get_connection_data_or_return!(ctx, msg);
   let manager_lock = get_manager_lock(ctx);
   let mut manager = manager_lock.lock();

   match manager.get_mut(connect_to.guild) {
      Some(handler) => handler.stop(),
      None => warn!("Could not load audio handler to stop"),
   };
}

pub fn join_message_and_play(ctx: Context, msg: Message, source: Box<dyn voice::AudioSource>, volume: f32) {
   let connect_to = get_connection_data_or_return!(ctx, msg);
   join_and_play(ctx, connect_to.guild, connect_to.channel, source, volume);
}

pub fn join_message(ctx: Context, msg: Message) {
   let connect_to = get_connection_data_or_return!(ctx, msg);
   let manager_lock = get_manager_lock(ctx);
   let mut manager = manager_lock.lock();

   match manager.join(connect_to.guild, connect_to.channel) {
      Some(_) => (),
      None => error!("Could not load audio handler for playback"),
   };
}
