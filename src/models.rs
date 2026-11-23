#![allow(dead_code)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    #[serde(alias = "Ftp", alias = "FTP")]
    Ftp,
    #[serde(alias = "Sftp", alias = "SFTP")]
    Sftp,
    #[serde(alias = "Ftps", alias = "FTPS")]
    Ftps,
}

impl Protocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Protocol::Sftp => "sftp",
            Protocol::Ftp => "ftp",
            Protocol::Ftps => "ftps",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Protocol::Sftp => "SFTP - SSH Dosya Aktarım Protokolü (Port 22)",
            Protocol::Ftp => "FTP - Standart Dosya Aktarım Protokolü (Port 21)",
            Protocol::Ftps => "FTPS - Güvenli TLS Şifrelemeli FTP (Port 21)",
        }
    }

    pub fn default_port(&self) -> u16 {
        match self {
            Protocol::Sftp => 22,
            Protocol::Ftp | Protocol::Ftps => 21,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "sftp" => Protocol::Sftp,
            "ftps" => Protocol::Ftps,
            _ => Protocol::Ftp,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerProfile {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: Option<String>,
    pub proto: Protocol,
    pub default_remote_dir: Option<String>,
    pub default_local_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileItem {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified_time: i64,
    pub permissions: String,
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDirection {
    Download,
    Upload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferStatus {
    Queued,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct TransferItem {
    pub id: String,
    pub source_path: String,
    pub target_path: String,
    pub direction: TransferDirection,
    pub size: u64,
    pub transferred_bytes: u64,
    pub progress: f64,
    pub speed_str: String,
    pub status: TransferStatus,
    pub error_message: Option<String>,
}
