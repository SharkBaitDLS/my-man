use log::error;
use serenity::{builder::EditRole, client::Context, model::prelude::GuildId};
use std::{
   fs::File,
   io::{ErrorKind, Read, Write},
   path::PathBuf,
};

pub async fn create_admin_role(ctx: &Context, guild_id: &GuildId, mut path: PathBuf) {
   path.push(".role_id");

   let admin_role_id;
   {
      let mut admin_role_data = String::new();
      if let Err(err) = File::open(&path).map(|mut file| file.read_to_string(&mut admin_role_data)) {
         if err.kind() != ErrorKind::NotFound {
            error!("Could not retrieve role ID for guild {:?}: {:?}", guild_id, err);
         }
      }
      if admin_role_data.is_empty() {
         admin_role_id = None;
      } else {
         admin_role_id = admin_role_data
            .parse::<u64>()
            .map_err(|err| error!("Could not parse .role_id for {:?}: {:?}", guild_id, err))
            .ok()
      }
   }

   if admin_role_id.is_none()
      || !guild_id
         .roles(&ctx)
         .await
         .unwrap() // we're okay with this because it only errors if the bot isn't in the guild
         .into_keys()
         .any(|role_id| role_id == admin_role_id.unwrap())
   {
      match File::create(&path) {
         Ok(mut file) => match guild_id
            .create_role(&ctx, EditRole::new().name("Sound Clip Admin"))
            .await
         {
            Ok(role) => {
               if let Err(err) = file.write_all(role.id.to_string().as_bytes()) {
                  error!(
                     "Could not write .role data for guild: {:?}, role ID: {:?}: {:?}",
                     guild_id, role.id, err
                  );
               }
            }
            Err(err) => error!("Could not create role for guild {:?}: {:?}", guild_id, err),
         },
         Err(err) => error!(
            "Could not create .role file for guild, not creating role: {:?}: {:?}",
            guild_id, err
         ),
      }
   }
}
