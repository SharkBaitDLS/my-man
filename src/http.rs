use rocket::{catch, http::Status, post, Request, State};
use serenity::model::id::{GuildId, UserId};

use crate::{
   audio::{connection_data::ConnectionData, playback::play_file_with_manager},
   WebContext,
};

#[catch(default)]
pub fn default_catcher(_status: Status, _request: &Request) {}

#[post("/play/<guild_id>/<user_id>/<name>")]
pub async fn play(ctx: &State<WebContext>, guild_id: u64, user_id: u64, name: &str) -> Result<(), Status> {
   if let Ok(user) = UserId(user_id).to_user(&ctx.cache_http).await {
      if let Some(connect_to) = ConnectionData::try_from_guild_user(&ctx.cache_http.cache, GuildId(guild_id), &user) {
         return match play_file_with_manager(ctx.songbird.clone(), name, connect_to)
            .await
            .underlying_error
         {
            Some(_) => Err(Status::InternalServerError),
            None => Ok(()),
         };
      };
   }
   Err(Status::NotFound)
}
