use crate::chat;
use crate::event::util::*;
use crate::playback;
use crate::util::log_on_error;
use async_trait::async_trait;
use log::debug;
use metrics::counter;
use serenity::{
   client::{Context, EventHandler},
   model::{channel::Message, channel::MessageType, gateway::Activity, gateway::Ready, id::GuildId, voice::VoiceState},
};

pub struct SoundboardListener;

#[async_trait]
impl EventHandler for SoundboardListener {
   async fn ready(&self, ctx: Context, ready: Ready) {
      debug!("{} is connected!", ready.user.name);
      // Discord's API doesn't support custom statuses: https://github.com/discord/discord-api-docs/issues/1160
      ctx.set_activity(Activity::listening("messages, type \"?help\" in chat"))
         .await;
      debug!("{:?} guilds are unavailable", ctx.cache.unavailable_guilds().await);
   }

   async fn voice_state_update(
      &self, ctx: Context, guild_id: Option<GuildId>, old: Option<VoiceState>, new: VoiceState,
   ) {
      match new.channel_id {
         Some(channel_id) if moved_to_non_afk(&ctx, guild_id.unwrap(), channel_id, old.and_then(|o| o.channel_id)) => {
            play_entrance(ctx, guild_id.unwrap(), channel_id, new.user_id).await
         }
         _ => move_if_last_user(ctx, guild_id).await,
      }
   }

   async fn message(&self, ctx: Context, msg: Message) {
      if let MessageType::Regular = msg.kind {
         if msg.content.starts_with('?') {
            if !msg.is_private() {
               log_on_error(msg.delete(&ctx)).await;
            }
            match msg.content.as_ref() {
               "?help" => log_on_error(msg.author.dm(ctx, chat::help)).await,
               "?list" => {
                  let content = chat::list(&ctx, &msg.author).await;
                  log_on_error(msg.author.dm(&ctx, |dm| dm.content(content))).await
               }
               "?stop" => playback::stop(ctx, msg).await,
               "?summon" => {
                  let mut my_man_msg = msg;
                  my_man_msg.content = "?myman".to_string();
                  play_file(ctx, my_man_msg).await
               }
               content if content.starts_with("?yt ") => play_youtube(ctx, msg).await,
               _ => {
                  counter!("sound_request", 1, "name" => get_file_name(&msg).to_string());
                  play_file(ctx, msg).await
               }
            };
         }
      }
   }
}
