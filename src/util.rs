use log::error;

pub fn log_on_error<T>(result: serenity::Result<T>) {
   if let Err(why) = result {
      error!("Failed discord call: {:?}", why)
   }
}
