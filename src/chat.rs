use crate::util::log_on_error;
use serenity::{builder::CreateMessage, client::Context, model::channel::Message};
use std::{collections::BinaryHeap, env, fs::read_dir};

pub fn help<'a, 'b>(msg: &'a mut CreateMessage<'b>) -> &'a mut CreateMessage<'b> {
   return msg.content(
      "You can type any of the following commands:
```
?list             - Returns a list of available sound files.
?soundFileName    - Plays the specified sound from the list.
?yt youtubeLink   - Plays the youtube link specified.
?stop             - Stops the sound that is currently playing.
?summon           - Summon the bot to your channel.
```",
   );
}

pub fn list<'a, 'b>(msg: &'a mut CreateMessage<'b>) -> &'a mut CreateMessage<'b> {
   let file_dir = env::var("AUDIO_FILE_DIR").expect("Audio file directory must be in the environment!");
   let file_names: BinaryHeap<String> = read_dir(file_dir)
      .unwrap()
      .map(|path| String::from(path.unwrap().path().file_stem().unwrap().to_str().unwrap()))
      .collect();
   let list_message = file_names.into_sorted_vec().into_iter().fold(
      String::from("Type any of the following into the chat to play the sound:\n```\n"),
      |accum, path| accum + "?" + &path + "\n",
   );
   return msg.content(list_message + "```");
}

pub fn dm_not_found(ctx: &Context, msg: &Message, name: &String) {
   log_on_error(
      msg.author
         .direct_message(ctx, |m| m.content(format!("Cannot find audio file for {}", name))),
   );
}
