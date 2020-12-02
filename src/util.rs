use log::error;
use std::future::Future;

pub async fn log_on_error<T>(result: impl Future<Output = serenity::Result<T>>) {
   if let Err(why) = result.await {
      error!("Failed discord call: {:?}", why)
   };
}
