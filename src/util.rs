use crate::playback::CallResult;
use log::error;

pub fn log_error_if_any(result: CallResult) -> CallResult {
   if let Some(ref err) = result.underlying_error {
      error!("Unexpected error occured during call: {}", err);
   }
   result
}
