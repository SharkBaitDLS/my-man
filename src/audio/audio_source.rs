use serenity::model::id::GuildId;
use songbird::input::{File as AudioFile, Input};
use std::{
   env,
   io::{Error, ErrorKind},
   path::{Component, PathBuf},
};

pub fn file(name: &str, guild_id: GuildId) -> Result<Input, Error> {
   get_path(name, guild_id).map(|path| AudioFile::new(path).into())
}

fn get_path(name: &str, guild_id: GuildId) -> Result<PathBuf, Error> {
   let file_dir = env::var("AUDIO_FILE_DIR").expect("Audio file directory must be in the environment!");
   let path: PathBuf = [
      file_dir,
      Into::<u64>::into(guild_id).to_string(),
      name.to_lowercase() + ".mp3",
   ]
   .iter()
   .collect();

   if path.components().any(|component| component == Component::ParentDir) {
      return Err(Error::new(
         ErrorKind::PermissionDenied,
         "Attempt to traverse directory hierarchy",
      ));
   }

   Ok(path)
}

#[cfg(test)]
mod tests {
   use super::*;
   use std::{
      fs::{self, File},
      io::{Error, ErrorKind, Read, Write},
   };
   use tempfile::{TempDir, tempdir};

   #[test]
   #[should_panic(expected = "Audio file directory must be in the environment!")]
   #[allow(unused_must_use)]
   fn test_path_requires_dir() {
      get_path("some_clip", GuildId::new(1));
   }

   #[test]
   fn test_guild_clip_retrieved() -> Result<(), Error> {
      let dir = setup_temp_directories()?;

      let mut file = File::open(get_path("clip", GuildId::new(1))?)?;
      let mut content = String::new();
      file.read_to_string(&mut content)?;

      assert_eq!(content, "first guild clip");

      file = File::open(get_path("clip", GuildId::new(2))?)?;
      content = String::new();
      file.read_to_string(&mut content)?;

      assert_eq!(content, "second guild clip");

      dir.close()?;
      Ok(())
   }

   #[test]
   fn test_relative_path_traversal_disallowed() -> Result<(), Error> {
      let dir = setup_temp_directories()?;

      match get_path("../2/clip", GuildId::new(1)) {
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

      unsafe { env::set_var("AUDIO_FILE_DIR", dir.path().as_os_str()) };
      Ok(dir)
   }
}
