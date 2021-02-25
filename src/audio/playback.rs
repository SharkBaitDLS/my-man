use crate::util::log_on_error;
use futures::executor::block_on;
use futures::stream::{FuturesOrdered, StreamExt};
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

pub async fn get_manager_lock(ctx: Context) -> Arc<Mutex<ClientVoiceManager>> {
   ctx.data
      .read()
      .await
      .get::<VoiceManager>()
      .cloned()
      .expect("Expected VoiceManager in data map")
}

pub struct ConnectionData {
   pub guild: GuildId,
   channel: ChannelId,
}

pub async fn get_connection_data_from_message(ctx: &Context, msg: &Message) -> Option<ConnectionData> {
   let possible_guilds = match msg.guild(&ctx.cache).await {
      Some(guild) => vec![guild],
      None => {
         ctx.cache
            .current_user()
            .await
            .guilds(&ctx.http)
            .await
            .unwrap_or_else(|err| {
               error!("Error retrieving this bot's guilds: {}", &err);
               Vec::new()
            })
            .into_iter()
            .map(|info| info.id.to_guild_cached(&ctx.cache))
            .collect::<FuturesOrdered<_>>()
            .filter_map(|guild| async { guild })
            .collect::<Vec<_>>()
            .await
      }
   };

   possible_guilds.into_iter().find_map(|guild| {
      match guild
         .voice_states
         .get(&msg.author.id)
         .and_then(|state| state.channel_id)
      {
         Some(channel_id) => Some(ConnectionData {
            guild: guild.id,
            channel: channel_id,
         }),
         None => {
            block_on(async {
               log_on_error(
                  msg.author
                     .direct_message(ctx, |m| m.content("You are not in a voice channel!")),
               )
               .await;
            });
            None
         }
      }
   })
}

async fn play_source(handler: &mut Handler, source: Box<dyn voice::AudioSource>, volume: f32) {
   let safe_audio: LockedAudio = handler.play_returning(source);
   {
      let mut audio = safe_audio.lock().await;
      audio.volume(volume);
   }
}

pub async fn join_and_play(
   ctx: Context, guild_id: GuildId, channel_id: ChannelId, source: Box<dyn voice::AudioSource>, volume: f32,
) {
   let manager_lock = get_manager_lock(ctx).await;
   let mut manager = manager_lock.lock().await;

   match manager.get_mut(guild_id) {
      Some(handler) => {
         if handler.channel_id != Some(channel_id) {
            handler.join(channel_id);
            // the underlying HTTP request to Discord's API to switch channels
            // doesn't immediately take effect, so the call above returning doesn't actually
            // mean the switch has happened, so sleep a bit to make sure people hear the whole clip
            sleep(Duration::from_secs(1));
         }
         play_source(handler, source, volume).await;
      }
      None => match manager.join(guild_id, channel_id) {
         Some(handler) => {
            sleep(Duration::from_secs(1));
            play_source(handler, source, volume).await;
         }
         None => error!("Could not create audio handler for initial join"),
      },
   };
}

pub async fn stop(ctx: Context, msg: Message) {
   if let Some(connect_to) = get_connection_data_from_message(&ctx, &msg).await {
      let manager_lock = get_manager_lock(ctx).await;
      let mut manager = manager_lock.lock().await;

      match manager.get_mut(connect_to.guild) {
         Some(handler) => handler.stop(),
         None => warn!("Could not load audio handler to stop"),
      }
   };
}

pub async fn join_connection_and_play(
   ctx: Context, connect_to: ConnectionData, source: Box<dyn voice::AudioSource>, volume: f32,
) {
   join_and_play(ctx, connect_to.guild, connect_to.channel, source, volume).await
}

pub async fn join_message_and_play(ctx: Context, msg: Message, source: Box<dyn voice::AudioSource>, volume: f32) {
   if let Some(connect_to) = get_connection_data_from_message(&ctx, &msg).await {
      join_connection_and_play(ctx, connect_to, source, volume).await;
   };
}
