use log::error;
use serenity::model::id::GuildId;
use songbird::ffmpeg;
use songbird::input::Input;
use songbird::ytdl;
use std::{env, fs::File, io::ErrorKind, path::Path};

pub async fn file<F>(name: &str, guild_id: &GuildId, not_found_handler: F) -> Option<Input>
where
   F: Fn(&str),
{
   let file_dir = env::var("AUDIO_FILE_DIR").expect("Audio file directory must be in the environment!");
   // TODO: platform agnostic paths
   // TODO: handle directory traversal attacks
   let audio_file_path_str = file_dir + "/" + &guild_id.as_u64().to_string() + "/" + &name.to_lowercase() + ".mp3";
   let path = Path::new(&audio_file_path_str);

   match File::open(&path).err() {
      Some(err) => {
         match err.kind() {
            ErrorKind::NotFound => not_found_handler(name),
            _ => error!("couldn't open {}: {}", audio_file_path_str, err.to_string()),
         };
         None
      }
      None => ffmpeg(path)
         .await
         .map_err(|err| error!("Err starting source: {:?}", err))
         .ok(),
   }
}

pub async fn youtube(url: &str) -> Option<Input> {
   match ytdl(url).await {
      Ok(source) => Option::from(source),
      Err(why) => {
         error!("Err starting source: {:?}", why);
         None
      }
   }
}
