use serde::{Deserialize, Serialize};
use std::time::Duration as StdDuration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenWrtConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub interface: String,
    pub private_key_path: Option<String>,
}

impl Default for OpenWrtConfig {
    fn default() -> Self {
        Self {
            host: "192.168.1.1".to_string(),
            port: 22,
            username: "root".to_string(),
            interface: "wan".to_string(),
            private_key_path: Some("~/.ssh/local".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ipv4Address {
    pub address: String,
    pub mask: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub target: String,
    pub mask: u8,
    pub nexthop: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceStatus {
    pub up: bool,
    pub pending: bool,
    pub available: bool,
    pub autostart: bool,
    pub dynamic: bool,
    pub uptime: u64,
    pub l3_device: Option<String>,
    pub proto: Option<String>,
    pub updated: Vec<String>,
    pub metric: i32,
    pub dns_metric: i32,
    pub delegation: bool,
    #[serde(rename = "ipv4-address")]
    pub ipv4_address: Vec<Ipv4Address>,
    #[serde(rename = "ipv6-address")]
    pub ipv6_address: Vec<String>,
    #[serde(rename = "ipv6-prefix")]
    pub ipv6_prefix: Vec<String>,
    #[serde(rename = "ipv6-prefix-assignment")]
    pub ipv6_prefix_assignment: Vec<String>,
    pub route: Vec<Route>,
    #[serde(rename = "dns-server")]
    pub dns_server: Vec<String>,
    #[serde(rename = "dns-search")]
    pub dns_search: Vec<String>,
    pub neighbors: Vec<String>,
    pub inactive: Option<serde_json::Value>,
    pub data: serde_json::Value,
}

impl InterfaceStatus {
    pub fn format_uptime(&self) -> String {
        let duration = StdDuration::from_secs(self.uptime);
        let days = duration.as_secs() / 86400;
        let hours = (duration.as_secs() % 86400) / 3600;
        let minutes = (duration.as_secs() % 3600) / 60;
        let seconds = duration.as_secs() % 60;

        if days > 0 {
            format!("{}d {}h {}m {}s", days, hours, minutes, seconds)
        } else if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }

    // pub fn is_connected(&self) -> bool {
    //     self.up && self.available
    // }
}

#[derive(Debug)]
pub enum AppError {
    Json(serde_json::Error),
    Io(std::io::Error),
    Other(std::io::Error),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Json(e) => write!(f, "JSON parsing error: {}", e),
            AppError::Io(e) => write!(f, "I/O error: {}", e),
            AppError::Other(e) => write!(f, "Error: {}", e),
        }
    }
}

impl std::error::Error for AppError {}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::Json(err)
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err)
    }
}

impl From<std::string::FromUtf8Error> for AppError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        AppError::Other(std::io::Error::new(std::io::ErrorKind::InvalidData, err))
    }
}

pub async fn fetch_interface_status() -> Result<InterfaceStatus, AppError> {
    let config = OpenWrtConfig::default();
    let command = format!("ubus call network.interface.{} status", config.interface);

    // Build SSH command arguments
    let mut args = vec![
        "-o".to_string(),
        "StrictHostKeyChecking=no".to_string(),
        "-o".to_string(),
        "UserKnownHostsFile=/dev/null".to_string(),
    ];

    // Add identity file if private key path is specified
    if let Some(private_key) = &config.private_key_path {
        args.push("-i".to_string());
        args.push(private_key.clone());
    }

    // Add username and host
    args.push(format!("{}@{}", config.username, config.host));

    // Add the command to execute
    args.push(command);

    // For now, let's implement a simple version using tokio::process::Command to run ssh
    // This is a temporary implementation until we get the russh client working properly
    let output = tokio::process::Command::new("ssh")
        .args(&args)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Other(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("SSH command failed: {}", stderr),
        )));
    }

    let stdout = String::from_utf8(output.stdout)?;
    let status: InterfaceStatus = serde_json::from_str(&stdout)?;
    Ok(status)
}
