use log::error;

pub struct CallResult {
   pub user_message: String,
   pub underlying_error: Option<String>,
}

impl CallResult {
   pub fn success<T: ToString>(user_message: T) -> Self {
      Self {
         user_message: user_message.to_string(),
         underlying_error: None,
      }
   }

   pub fn failure<T: ToString, U: ToString>(user_message: T, underlying_error: U) -> Self {
      Self {
         user_message: user_message.to_string(),
         underlying_error: Some(underlying_error.to_string()),
      }
   }
}

pub fn log_error_if_any(result: CallResult) -> CallResult {
   if let Some(ref err) = result.underlying_error {
      error!("Unexpected error occured during call: {}", err);
   }
   result
}
