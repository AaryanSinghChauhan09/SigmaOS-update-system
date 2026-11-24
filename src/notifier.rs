use crate::error::UpdateError;

pub struct UpdateNotifier;

impl UpdateNotifier {
    pub fn new() -> Result<Self, UpdateError> {
        Ok(Self)
    }
    
    pub fn notify_update(&self, message: &str) -> Result<(), UpdateError> {
        // Send desktop notification
        let status = std::process::Command::new("notify-send")
            .arg("SigmaOS Update")
            .arg(message)
            .status()?;
        
        if !status.success() {
            log::warn!("Failed to send notification");
        }
        
        Ok(())
    }
    
    pub fn notify_error(&self, message: &str) -> Result<(), UpdateError> {
        // Send error notification
        let status = std::process::Command::new("notify-send")
            .arg("--urgency=critical")
            .arg("SigmaOS Update Error")
            .arg(message)
            .status()?;
        
        if !status.success() {
            log::warn!("Failed to send error notification");
        }
        
        Ok(())
    }
}
