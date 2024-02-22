use futures::{stream, StreamExt};
use serenity::{
   client::Context,
   model::{id::GuildId, user::User},
};
use std::env;

use crate::guilds::{get_bot_guild_infos, get_guild};

pub async fn list(ctx: &Context, maybe_guild_id: Option<GuildId>, author: &User) -> String {
   let author_guilds = if let Some(guild) = maybe_guild_id.and_then(|id| get_guild(ctx, id)) {
      vec![guild]
   } else {
      let guilds = get_bot_guild_infos(ctx).await;
      stream::iter(guilds.iter().map(|id| get_guild(ctx, id)))
         .filter_map(|maybe_guild| async {
            if let Some(guild) = maybe_guild {
               if guild.member(ctx, &author.id).await.is_ok() {
                  Some(guild)
               } else {
                  None
               }
            } else {
               None
            }
         })
         .collect::<Vec<_>>()
         .await
   };

   let web_uri = env::var("WEB_URI").expect("Expected a web URI in the environment");
   let mut content: String = String::new();
   if author_guilds.is_empty() {
      content.push_str("You have no mutual servers with this bot");
   }
   author_guilds.iter().for_each(|guild| {
      content.push_str(&format!("[**{}**]({}/clips/{})\n", guild.name, web_uri, guild.id));
   });

   content
}
