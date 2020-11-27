use crate::util::log_on_error;
use log::error;
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

pub fn list<'a, 'b>(msg: &'a mut CreateMessage<'b>) -> &'a mut CreateMessage<'b> {
   let file_dir = env::var("AUDIO_FILE_DIR").expect("Audio file directory must be in the environment!");
   let file_names = read_dir(file_dir)
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
      msg.content("No MP3 files found for playback in the configured directory!")
   } else {
      let list_message = file_names.into_sorted_vec().into_iter().fold(
         String::from("Type any of the following into chat to play the sound:\n```\n"),
         |accum, path| accum + "?" + &path + "\n",
      );
      msg.content(list_message + "```")
   }
}

pub fn dm_not_found(ctx: &Context, msg: &Message, name: &str) {
   log_on_error(
      msg.author
         .direct_message(ctx, |m| m.content(format!("Cannot find audio file for {}", name))),
   )
}
