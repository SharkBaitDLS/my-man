use futures::Future;
use log::error;
use serenity::model::id::GuildId;
use songbird::ffmpeg;
use songbird::input::Input;
use songbird::ytdl;
use std::ffi::OsStr;
use std::io::Error;
use std::path::PathBuf;
use std::{env, fs::File, io::ErrorKind};

pub async fn file<F>(name: &str, guild_id: &GuildId, not_found_handler: F) -> Option<Input>
where
   F: Fn(&str),
{
   match get_path(name, guild_id).await {
      Err(err) => {
         match err.kind() {
            ErrorKind::NotFound => not_found_handler(name),
            _ => error!("couldn't open {}: {}", name, err.to_string()),
         };
         None
      }
      Ok(path) => play_path(path).await,
   }
}

pub async fn file_async_nf<'a, F, Fut>(name: &'a str, guild_id: &GuildId, not_found_handler: F) -> Option<Input>
where
   F: Fn(&'a str) -> Fut,
   Fut: Future<Output = ()>,
{
   match get_path(name, guild_id).await {
      Err(err) => {
         match err.kind() {
            ErrorKind::NotFound => not_found_handler(name).await,
            _ => error!("couldn't open {}: {}", name, err.to_string()),
         };
         None
      }
      Ok(path) => play_path(path).await,
   }
}

async fn get_path(name: &str, guild_id: &GuildId) -> Result<PathBuf, Error> {
   let file_dir = env::var("AUDIO_FILE_DIR").expect("Audio file directory must be in the environment!");
   // TODO: platform agnostic paths
   // TODO: handle directory traversal attacks
   let mut path = PathBuf::new();
   path.push(file_dir + "/" + &guild_id.as_u64().to_string() + "/" + &name.to_lowercase() + ".mp3");

   File::open(&path).map(|_| path)
}

async fn play_path<P: AsRef<OsStr>>(path: P) -> Option<Input> {
   ffmpeg(path)
      .await
      .map_err(|err| error!("Err starting source: {:?}", err))
      .ok()
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
