#![forbid(unsafe_code)]

pub mod updater;
pub mod error;
pub mod channel;
pub mod notifier;

pub use updater::UpdateManager;
pub use error::UpdateError;
pub use channel::UpdateChannel;
pub use notifier::UpdateNotifier;
