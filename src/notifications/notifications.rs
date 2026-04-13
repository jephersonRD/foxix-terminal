use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
    Success,
}

impl NotificationLevel {
    pub fn urgency(&self) -> &'static str {
        match self {
            Self::Info => "low",
            Self::Warning => "normal",
            Self::Error => "critical",
            Self::Success => "low",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Info => "dialog-information",
            Self::Warning => "dialog-warning",
            Self::Error => "dialog-error",
            Self::Success => "dialog-information",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub title: String,
    pub body: String,
    pub level: NotificationLevel,
    pub icon: Option<String>,
    pub timeout: u64,
    pub created_at_secs: u64,
    pub action: Option<String>,
}

impl Notification {
    pub fn new(
        title: impl Into<String>,
        body: impl Into<String>,
        level: NotificationLevel,
    ) -> Self {
        Self {
            id: uuid_simple(),
            title: title.into(),
            body: body.into(),
            level,
            icon: None,
            timeout: 5,
            created_at_secs: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            action: None,
        }
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout = timeout_secs;
        self
    }

    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }
}

pub struct NotificationManager {
    notifications: Arc<RwLock<Vec<Notification>>>,
    enabled: bool,
    max_notifications: usize,
    default_timeout: u64,
    use_dbus: bool,
    use_notify_send: bool,
}

impl NotificationManager {
    pub fn new() -> Self {
        let use_notify_send = which::which("notify-send").is_ok();

        Self {
            notifications: Arc::new(RwLock::new(Vec::new())),
            enabled: true,
            max_notifications: 50,
            default_timeout: 5,
            use_dbus: false,
            use_notify_send,
        }
    }

    pub fn send(&self, notification: Notification) -> Result<String, String> {
        if !self.enabled {
            return Err("Notifications disabled".to_string());
        }

        let id = notification.id.clone();

        if self.use_notify_send {
            self.send_via_notify_send(&notification)?;
        } else if self.use_dbus {
            self.send_via_dbus(&notification)?;
        } else {
            log::warn!("No notification backend available");
            return Err("No notification backend".to_string());
        }

        let mut notifications = self.notifications.write();
        if notifications.len() >= self.max_notifications {
            notifications.remove(0);
        }
        notifications.push(notification);

        Ok(id)
    }

    pub fn send_simple(
        &self,
        title: &str,
        body: &str,
        level: NotificationLevel,
    ) -> Result<String, String> {
        let notification = Notification::new(title, body, level);
        self.send(notification)
    }

    pub fn send_info(&self, title: &str, body: &str) -> Result<String, String> {
        self.send_simple(title, body, NotificationLevel::Info)
    }

    pub fn send_warning(&self, title: &str, body: &str) -> Result<String, String> {
        self.send_simple(title, body, NotificationLevel::Warning)
    }

    pub fn send_error(&self, title: &str, body: &str) -> Result<String, String> {
        self.send_simple(title, body, NotificationLevel::Error)
    }

    pub fn send_success(&self, title: &str, body: &str) -> Result<String, String> {
        self.send_simple(title, body, NotificationLevel::Success)
    }

    fn send_via_notify_send(&self, notification: &Notification) -> Result<(), String> {
        let mut cmd = Command::new("notify-send");
        cmd.arg("-u").arg(notification.level.urgency());
        cmd.arg("-a").arg("foxix");
        cmd.arg("-t").arg((notification.timeout * 1000).to_string());

        if let Some(ref icon) = notification.icon {
            cmd.arg("-i").arg(icon);
        } else {
            cmd.arg("-i").arg(notification.level.icon());
        }

        cmd.arg(&notification.title);
        cmd.arg(&notification.body);

        let output = cmd.output().map_err(|e| e.to_string())?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(stderr.to_string());
        }

        Ok(())
    }

    fn send_via_dbus(&self, _notification: &Notification) -> Result<(), String> {
        Ok(())
    }

    pub fn get_notifications(&self) -> Vec<Notification> {
        self.notifications.read().clone()
    }

    pub fn clear_notifications(&self) {
        self.notifications.write().clear();
    }

    pub fn remove_notification(&self, id: &str) -> bool {
        let mut notifications = self.notifications.write();
        if let Some(pos) = notifications.iter().position(|n| n.id == id) {
            notifications.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn toggle_enabled(&mut self) -> bool {
        self.enabled = !self.enabled;
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_default_timeout(&mut self, timeout_secs: u64) {
        self.default_timeout = timeout_secs;
    }

    pub fn set_max_notifications(&mut self, max: usize) {
        self.max_notifications = max;
    }

    pub fn cleanup_old(&self, max_age_secs: u64) {
        let mut notifications = self.notifications.write();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        notifications.retain(|n| now - n.created_at_secs < max_age_secs);
    }
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{:x}{:x}", timestamp.as_secs(), timestamp.subsec_nanos())
}
