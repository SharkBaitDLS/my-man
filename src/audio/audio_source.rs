use serenity::model::id::GuildId;
use songbird::ffmpeg;
use songbird::input::{error::Error, Input};
use std::path::PathBuf;
use std::{env, fs::File};

pub async fn file(name: &str, guild_id: &GuildId) -> Result<Input, Error> {
   match get_path(name, guild_id).await {
      Ok(path) => ffmpeg(path).await,
      Err(err) => Err(Error::Io(err)),
   }
}

async fn get_path(name: &str, guild_id: &GuildId) -> Result<PathBuf, std::io::Error> {
   let file_dir = env::var("AUDIO_FILE_DIR").expect("Audio file directory must be in the environment!");
   // TODO: platform agnostic paths
   // TODO: handle directory traversal attacks
   let mut path = PathBuf::new();
   path.push(file_dir + "/" + &guild_id.as_u64().to_string() + "/" + &name.to_lowercase() + ".mp3");

   File::open(&path).map(|_| path)
}
