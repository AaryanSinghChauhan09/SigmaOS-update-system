#![forbid(unsafe_code)]

pub mod channel;
pub mod error;
pub mod notifier;
pub mod updater;

pub use channel::UpdateChannel;
pub use error::UpdateError;
pub use notifier::UpdateNotifier;
pub use updater::UpdateManager;
