use log::error;
use serenity::client::Context;
use std::{
   env,
   fs::File,
   io::{ErrorKind, Read, Write},
   path::PathBuf,
};

use crate::guilds;

pub async fn create_admin_roles(ctx: &Context) {
   for guild in guilds::get_bot_guild_infos(ctx).await {
      let file_dir = env::var("AUDIO_FILE_DIR").expect("Audio file directory must be in the environment!");
      let path: PathBuf = [file_dir, guild.id.as_u64().to_string(), ".role_id".to_string()]
         .iter()
         .collect();

      let admin_role_id;
      {
         let mut admin_role_data = String::new();
         if let Err(err) = File::open(&path).map(|mut file| file.read_to_string(&mut admin_role_data)) {
            if err.kind() != ErrorKind::NotFound {
               error!("Could not retrieve role ID for guild {:?}: {:?}", guild.id, err);
            }
         }
         if admin_role_data.is_empty() {
            admin_role_id = None;
         } else {
            admin_role_id = admin_role_data
               .parse::<u64>()
               .map_err(|err| error!("Could not parse .role_id for {:?}: {:?}", guild.id, err))
               .ok()
         }
      }

      if admin_role_id.is_none()
         || !guild
            .id
            .roles(&ctx)
            .await
            .unwrap() // we're okay with this because it only errors if the bot isn't in the guild
            .into_keys()
            .any(|role_id| role_id == admin_role_id.unwrap())
      {
         match File::create(&path) {
            Ok(mut file) => match guild.id.create_role(&ctx, |role| role.name("Sound Clip Admin")).await {
               Ok(role) => {
                  if let Err(err) = file.write_all(role.id.to_string().as_bytes()) {
                     error!(
                        "Could not write .role data for guild: {:?}, role ID: {:?}: {:?}",
                        guild.id, role.id, err
                     );
                  }
               }
               Err(err) => error!("Could not create role for guild {:?}: {:?}", guild.id, err),
            },
            Err(err) => error!(
               "Could not create .role file for guild, not creating role: {:?}: {:?}",
               guild.id, err
            ),
         }
      }
   }
}
