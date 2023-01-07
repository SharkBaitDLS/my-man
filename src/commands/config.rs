use log::{error, info};
use serenity::{
   client::Context,
   model::application::command::{Command, CommandOptionType},
};

#[derive(Clone, Debug)]
pub struct CommandOption<'a> {
   pub name: &'a str,
   pub description: &'a str,
   pub kind: CommandOptionType,
   pub required: bool,
}

impl Default for CommandOption<'_> {
   fn default() -> Self {
      Self {
         name: Default::default(),
         description: Default::default(),
         kind: CommandOptionType::String,
         required: false,
      }
   }
}

#[derive(Clone, Debug, Default)]
pub struct CommandConfig<'a> {
   pub name: &'a str,
   pub description: &'a str,
   pub options: Vec<CommandOption<'a>>,
}

impl CommandConfig<'_> {
   pub fn is_equivalent(&self, command: &Command) -> bool {
      command.name == self.name
         && command.description == self.description
         && command.options.len() == self.options.len()
         && command.options.iter().all(
            |option| match self.options.iter().find(|config| config.name == option.name) {
               Some(config) => {
                  option.description == config.description
                     && option.kind == config.kind
                     && option.required == config.required
               }
               None => false,
            },
         )
   }

   pub async fn register_command(&self, ctx: &Context) {
      info!("Registering command: {:?}", &self);
      if let Err(err) = Command::create_global_application_command(&ctx, |new| {
         let mut created = new;
         for option in &self.options {
            created = created.create_option(|new_option| {
               new_option
                  .name(option.name)
                  .description(option.description)
                  .kind(option.kind)
                  .required(option.required)
            })
         }
         created.name(self.name).description(self.description)
      })
      .await
      {
         error!("Could not register command: {:?}", err)
      }
   }
}
