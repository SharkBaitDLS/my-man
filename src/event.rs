use crate::audio::{audio_source, playback};
use crate::chat;
use crate::util::log_on_error;
use log::{debug, error, info};
use serenity::{
   client::{Context, EventHandler},
   model::{
      channel::Message, channel::MessageType, gateway::Activity, gateway::Ready, id::ChannelId, id::GuildId,
      id::UserId, user::User, voice::VoiceState,
   },
};

pub struct Listener;

fn is_not_afk(ctx: &Context, guild_id: GuildId, channel_id: ChannelId) -> bool {
   return guild_id
      .to_guild_cached(&ctx.cache)
      .and_then(|guild| guild.read().afk_channel_id)
      .map_or(true, |afk_id| afk_id != channel_id);
}

fn channel_changed(ctx: &Context, guild_id: GuildId, channel_id: ChannelId, old_id: Option<ChannelId>) -> bool {
   let moved_or_joined = old_id
      .map(|old_channel_id| old_channel_id != channel_id)
      .unwrap_or(true);

   return moved_or_joined && is_not_afk(ctx, guild_id, channel_id);
}

fn play_entrance(ctx: Context, guild_id: GuildId, channel_id: ChannelId, user_id: UserId) {
   match user_id.to_user(&ctx) {
      Ok(user) => match user {
         User { bot: true, .. } => debug!("A bot joined a channel: {}", user.name),
         _ => match audio_source::file(&user.name, |file| info!("No user sound file found for {}", file)) {
            Some(source) => playback::join_and_play(ctx, guild_id, channel_id, source, 1.0),
            None => (),
         },
      },
      Err(why) => error!("Could not get user name: {}", why.to_string()),
   }
}

fn move_if_last_user(ctx: Context, guild_id: Option<GuildId>) {
   match guild_id
      .and_then(|id| id.to_guild_cached(&ctx.cache))
      .map(|guild| guild.read().voice_states.len())
   {
      // the bot is the only one left in voice
      Some(1) => {
         let manager_lock = ctx
            .data
            .read()
            .get::<playback::VoiceManager>()
            .cloned()
            .expect("Expected VoiceManager in data map");
         let mut manager = manager_lock.lock();
         manager.leave(guild_id.unwrap());
      }
      _ => (),
   }
}

fn play_youtube(ctx: Context, msg: Message) {
   let url = msg.content.split_at(4).1.to_string();
   if !url.starts_with("http") {
      log_on_error(
         msg.author
            .direct_message(ctx, |m| m.content("You must provide a valid YouTube URL!")),
      );
      return;
   };
   match audio_source::youtube(&url) {
      Some(source) => playback::join_message_and_play(ctx, msg, source, 0.2),
      None => error!("Could not play youtube video at {}", url),
   }
}

fn play_file(ctx: Context, msg: Message) {
   let name = &msg.content.split_at(1).1.to_string();
   match audio_source::file(name, |name| chat::dm_not_found(&ctx, &msg, name)) {
      Some(source) => playback::join_message_and_play(ctx, msg, source, 1.0),
      None => (),
   }
}

impl EventHandler for Listener {
   fn ready(&self, ctx: Context, ready: Ready) {
      info!("{} is connected!", ready.user.name);
      ctx.set_activity(Activity::playing("Type ?help in chat"));
   }

   fn voice_state_update(&self, ctx: Context, guild_id: Option<GuildId>, old: Option<VoiceState>, new: VoiceState) {
      match new.channel_id {
         Some(channel_id) if channel_changed(&ctx, guild_id.unwrap(), channel_id, old.and_then(|o| o.channel_id)) => {
            play_entrance(ctx, guild_id.unwrap(), channel_id, new.user_id)
         }
         Some(_) => (),
         None => move_if_last_user(ctx, guild_id),
      }
   }

   fn message(&self, ctx: Context, msg: Message) {
      if let MessageType::Regular = msg.kind {
         if msg.content.starts_with("?") {
            if !msg.is_private() {
               log_on_error(msg.delete(&ctx));
            }
            match msg.content.as_ref() {
               "?help" => log_on_error(msg.author.direct_message(ctx, chat::help)),
               "?list" => log_on_error(msg.author.direct_message(ctx, chat::list)),
               "?stop" => playback::stop(ctx, msg),
               "?summon" => playback::join_message(ctx, msg),
               content if content.starts_with("?yt ") => play_youtube(ctx, msg),
               _ => play_file(ctx, msg),
            };
         }
      }
   }
}
