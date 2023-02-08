use serenity::model::id::GuildId;
use songbird::input::{error::Error, Input};
use std::{
   env,
   fs::File,
   io::ErrorKind,
   path::{Component, PathBuf},
};

pub async fn file(name: &str, guild_id: &GuildId) -> Result<Input, Error> {
   match get_path(name, guild_id).await {
      Ok(path) => songbird::ffmpeg(path).await,
      Err(err) => Err(Error::Io(err)),
   }
}

async fn get_path(name: &str, guild_id: &GuildId) -> Result<PathBuf, std::io::Error> {
   let file_dir = env::var("AUDIO_FILE_DIR").expect("Audio file directory must be in the environment!");
   let path: PathBuf = [file_dir, guild_id.as_u64().to_string(), name.to_lowercase() + ".mp3"]
      .iter()
      .collect();

   if path
      .components()
      .into_iter()
      .any(|component| component == Component::ParentDir)
   {
      return Err(std::io::Error::new(
         ErrorKind::PermissionDenied,
         "Attempt to traverse directory hierarchy",
      ));
   }

   File::open(&path).map(|_| path)
}

#[cfg(test)]
mod tests {
   use super::*;
   use futures::executor::block_on;
   use std::{
      fs,
      io::{ErrorKind, Read, Write},
   };
   use tempfile::{tempdir, TempDir};

   #[test]
   #[should_panic(expected = "Audio file directory must be in the environment!")]
   #[allow(unused_must_use)]
   fn test_path_requires_dir() {
      block_on(get_path("some_clip", &GuildId(1)));
   }

   #[test]
   fn test_guild_clip_retrieved() -> Result<(), Error> {
      let dir = setup_temp_directories()?;

      let mut file = File::open(block_on(get_path("clip", &GuildId(1)))?)?;
      let mut content = String::new();
      file.read_to_string(&mut content)?;

      assert_eq!(content, "first guild clip");

      file = File::open(block_on(get_path("clip", &GuildId(2)))?)?;
      content = String::new();
      file.read_to_string(&mut content)?;

      assert_eq!(content, "second guild clip");

      dir.close()?;
      Ok(())
   }

   #[test]
   fn test_relative_path_traversal_disallowed() -> Result<(), Error> {
      let dir = setup_temp_directories()?;

      match block_on(get_path("../2/clip", &GuildId(1))) {
         Err(err) => assert!(err.kind() == ErrorKind::PermissionDenied),
         Ok(path) => {
            let mut file = File::open(path)?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            panic!("Expected an error to be raised, got file content: {content}");
         }
      }

      dir.close()?;
      Ok(())
   }

   fn setup_temp_directories() -> Result<TempDir, Error> {
      let dir = tempdir()?;
      let first_guild = dir.path().join("1");
      let second_guild = dir.path().join("2");
      fs::create_dir(&first_guild)?;
      fs::create_dir(&second_guild)?;

      let mut first_guild_file = File::create(first_guild.join("clip.mp3"))?;
      first_guild_file.write_all(b"first guild clip")?;

      let mut another_first_guild_file = File::create(first_guild.join("another_clip.mp3"))?;
      another_first_guild_file.write_all(b"another first guild clip")?;

      let mut second_guild_file = File::create(second_guild.join("clip.mp3"))?;
      second_guild_file.write_all(b"second guild clip")?;

      let mut another_second_guild_file = File::create(second_guild.join("another_clip.mp3"))?;
      another_second_guild_file.write_all(b"another second guild file")?;

      env::set_var("AUDIO_FILE_DIR", dir.path().as_os_str());
      Ok(dir)
   }
}
