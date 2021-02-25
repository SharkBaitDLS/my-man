use crate::util::log_on_error;
use futures::executor::block_on;
use futures::stream::{FuturesOrdered, StreamExt};
use log::error;
use serenity::{
   client::{bridge::voice::VoiceGatewayManager, Context},
   model::{channel::Message, id::ChannelId, id::GuildId},
   prelude::Mutex,
   prelude::*,
};
use songbird::create_player;
use songbird::input::Input;
use songbird::Call;
use songbird::Songbird;
use std::sync::Arc;
use tokio::sync::MutexGuard;

pub struct VoiceManager;

impl TypeMapKey for VoiceManager {
   type Value = Arc<Mutex<dyn VoiceGatewayManager>>;
}

pub async fn get_manager(ctx: Context) -> Arc<Songbird> {
   songbird::get(&ctx)
      .await
      .expect("Songbird voice client should have been placed during initialization")
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

async fn play_source(mut call: MutexGuard<'_, Call>, source: Input, volume: f32) {
   let (mut track, _) = create_player(source);
   track.set_volume(volume);
   call.play(track);
}

pub async fn join_and_play(ctx: Context, guild_id: GuildId, channel_id: ChannelId, source: Input, volume: f32) {
   let manager = get_manager(ctx).await;

   match manager.join(guild_id, channel_id).await {
      (call, Ok(_)) => play_source(call.lock().await, source, volume).await,
      (_, Err(err)) => error!("Could not join to play audio: {}", err),
   }
}

pub async fn stop(ctx: Context, msg: Message) {
   if let Some(connect_to) = get_connection_data_from_message(&ctx, &msg).await {
      let manager = get_manager(ctx).await;

      if let Some(call) = manager.get(connect_to.guild) {
         call.lock().await.stop();
      };
   }
}

pub async fn join_connection_and_play(ctx: Context, connect_to: ConnectionData, source: Input, volume: f32) {
   join_and_play(ctx, connect_to.guild, connect_to.channel, source, volume).await
}

pub async fn join_message_and_play(ctx: Context, msg: Message, source: Input, volume: f32) {
   if let Some(connect_to) = get_connection_data_from_message(&ctx, &msg).await {
      join_connection_and_play(ctx, connect_to, source, volume).await;
   };
}
