use log::error;
use serenity::voice;
use std::{env, fs::File, io::ErrorKind, path::Path};

pub fn file<F>(name: &str, not_found_handler: F) -> Option<Box<dyn voice::AudioSource>>
where
   F: Fn(&str),
{
   let file_dir = env::var("AUDIO_FILE_DIR").expect("Audio file directory must be in the environment!");
   let audio_file_path_str = file_dir + &name.to_lowercase() + ".mp3";
   let path = Path::new(&audio_file_path_str);

   match File::open(&path) {
      Err(why) => {
         match why.kind() {
            ErrorKind::NotFound => not_found_handler(name),
            _ => error!("couldn't open {}: {}", audio_file_path_str, why.to_string()),
         };
         return None;
      }
      Ok(file) => file,
   };

   match voice::ffmpeg(path) {
      Ok(source) => Option::from(source),
      Err(why) => {
         error!("Err starting source: {:?}", why);
         None
      }
   }
}

pub fn youtube(url: &str) -> Option<Box<dyn voice::AudioSource>> {
   match voice::ytdl(url) {
      Ok(source) => Option::from(source),
      Err(why) => {
         error!("Err starting source: {:?}", why);
         None
      }
   }
}
