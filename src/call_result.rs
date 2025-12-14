use log::error;
use std::fmt::Display;

pub struct CallResult {
   pub user_message: String,
   pub underlying_error: Option<String>,
}

impl CallResult {
   pub fn success(user_message: String) -> Self {
      Self {
         user_message,
         underlying_error: None,
      }
   }

   pub fn failure<U: Display>(user_message: String, underlying_error: U) -> Self {
      Self {
         user_message,
         underlying_error: Some(underlying_error.to_string()),
      }
   }
}

pub fn log_error_if_any(result: CallResult) -> CallResult {
   if let Some(ref err) = result.underlying_error {
      error!("Unexpected error occured during call: {err}");
   }
   result
}
