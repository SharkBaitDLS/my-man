use log::error;
use serenity::Result as SerenityResult;

pub fn log_on_error<T>(result: SerenityResult<T>) {
   if let Err(why) = result {
      error!("Failed discord call: {:?}", why);
   }
}
