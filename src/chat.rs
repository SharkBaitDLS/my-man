use futures::{
   executor::block_on,
   stream::{FuturesOrdered, StreamExt},
};
use serenity::{
   client::Context,
   model::{id::GuildId, user::User},
};

pub async fn list(ctx: &Context, maybe_guild_id: Option<GuildId>, author: &User) -> String {
   let bot = ctx.cache.current_user().await;

   let author_guilds = if let Some(guild) = maybe_guild_id.and_then(|id| block_on(id.to_guild_cached(&ctx))) {
      vec![guild]
   } else if let Ok(guilds) = bot.guilds(ctx).await {
      guilds
         .iter()
         .map(|guild_id| ctx.cache.guild(guild_id))
         .collect::<FuturesOrdered<_>>()
         .filter_map(|maybe_guild| async {
            if let Some(guild) = maybe_guild {
               if guild.member(&ctx, &author.id).await.is_ok() {
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
   } else {
      Vec::new()
   };

   let mut content: String = String::new();
   if author_guilds.is_empty() {
      content.push_str("You have no mutual servers with this bot");
   }
   author_guilds.iter().for_each(|guild| {
      content.push_str(&format!(
         "[**{}**](https://soundboard.imgoodproductions.org/clips/{})\n",
         guild.name, guild.id
      ));
   });

   content
}
