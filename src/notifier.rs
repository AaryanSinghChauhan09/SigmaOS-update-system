use crate::error::UpdateError;

pub struct UpdateNotifier;

impl UpdateNotifier {
    pub fn new() -> Result<Self, UpdateError> {
        Ok(Self)
    }

    pub fn notify_update(&self, message: &str) -> Result<(), UpdateError> {
        // Send desktop notification — best-effort: headless systems
        // and CI have no notify-send, which must not fail updates.
        match std::process::Command::new("notify-send")
            .arg("SigmaOS Update")
            .arg(message)
            .status()
        {
            Ok(status) if !status.success() => log::warn!("notify-send exited with {}", status),
            Ok(_) => {}
            Err(e) => log::warn!("notify-send unavailable: {}", e),
        }
        Ok(())
    }

    pub fn notify_error(&self, message: &str) -> Result<(), UpdateError> {
        // Send error notification — same best-effort policy.
        match std::process::Command::new("notify-send")
            .arg("--urgency=critical")
            .arg("SigmaOS Update Error")
            .arg(message)
            .status()
        {
            Ok(status) if !status.success() => log::warn!("notify-send exited with {}", status),
            Ok(_) => {}
            Err(e) => log::warn!("notify-send unavailable: {}", e),
        }
        Ok(())
    }
}
