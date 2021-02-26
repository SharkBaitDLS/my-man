use crate::util::log_on_error;
use futures::stream::{FuturesOrdered, StreamExt};
use log::error;
use serenity::model::user::User;
use serenity::{builder::CreateMessage, client::Context, model::channel::Message};
use std::{collections::BinaryHeap, env, fs::read_dir};

pub fn help<'a, 'b>(msg: &'a mut CreateMessage<'b>) -> &'a mut CreateMessage<'b> {
   msg.content(
      "You can type any of the following commands:
```
?list             - Returns a list of available sound files.
?<soundFileName>  - Plays the specified sound from the list.
?yt <youtubeLink> - Plays the youtube link specified.
?stop             - Stops the currently playing sound(s).
?summon           - Summon the bot to your current voice channel.
```",
   )
}

pub async fn list(ctx: &Context, author: &User) -> String {
   let file_dir = env::var("AUDIO_FILE_DIR").expect("Audio file directory must be in the environment!");
   let bot = ctx.cache.current_user().await;

   let author_guilds = if let Ok(guilds) = bot.guilds(ctx).await {
      guilds
         .iter()
         .map(|guild_id| ctx.cache.guild(guild_id))
         .collect::<FuturesOrdered<_>>()
         .filter_map(|maybe_guild| async {
            if let Some(guild) = maybe_guild {
               if guild.member(&ctx, &author.id).await.is_ok() {
                  Some(guild)
               } else {
                  None
               }
            } else {
               None
            }
         })
         .collect::<Vec<_>>()
         .await
   } else {
      Vec::new()
   };

   let mut content = "Your commands per server:\n__**Servers**__\n".to_owned();

   author_guilds.iter().for_each(|guild| {
      // TODO: platform agnostic paths
      // TODO: handle directory traversal attacks
      let guild_dir = String::from(&file_dir) + "/" + &guild.id.as_u64().to_string();

      let file_names = read_dir(guild_dir)
         .map(|entries| {
            entries
               .filter_map(|maybe_entry| {
                  maybe_entry
                     .map(|entry| {
                        let path = entry.path();
                        path
                           .file_stem()
                           .filter(|_| path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("mp3"))
                           .and_then(|stem| stem.to_str())
                           .map(String::from)
                     })
                     .ok()
                     .flatten()
               })
               .collect()
         })
         .unwrap_or_else(|err| {
            error!("Could not list audio file directory: {}", err);
            BinaryHeap::new()
         });

      if file_names.is_empty() {
         content.push_str(&format!("**{}**\nNo commands available.\n", guild.name));
      } else {
         let list_message = file_names.into_sorted_vec().into_iter().fold(
            format!("**{}**\nCommands available:\n```\n", guild.name),
            |accum, path| accum + "?" + &path + "\n",
         );
         content.push_str(&(list_message + "```\n"));
      }
   });

   return content;
}

pub async fn dm_not_found(ctx: &Context, msg: &Message, name: &str) {
   log_on_error(
      msg.author
         .direct_message(ctx, |m| m.content(format!("Cannot find audio file for {}", name))),
   )
   .await;
}
