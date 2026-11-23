#![allow(dead_code)]
use std::fs;
use std::path::PathBuf;
use crate::models::ServerProfile;

pub struct ServerStore {
    path: PathBuf,
}

impl ServerStore {
    pub fn new() -> Self {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("shift");
        let _ = fs::create_dir_all(&path);
        path.push("servers.json");
        Self { path }
    }

    pub fn get_all(&self) -> Vec<ServerProfile> {
        if !self.path.exists() {
            return Vec::new();
        }
        match fs::read_to_string(&self.path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    pub fn save_all(&self, servers: &[ServerProfile]) -> bool {
        match serde_json::to_string_pretty(servers) {
            Ok(json) => fs::write(&self.path, json).is_ok(),
            Err(_) => false,
        }
    }

    pub fn add_server(&self, profile: ServerProfile) {
        let mut servers = self.get_all();
        // Remove existing with same host and user if exists
        servers.retain(|s| !(s.host == profile.host && s.user == profile.user));
        servers.insert(0, profile);
        self.save_all(&servers);
    }

    pub fn update_server(&self, profile: ServerProfile) {
        let mut servers = self.get_all();
        if let Some(idx) = servers.iter().position(|s| s.id == profile.id) {
            servers[idx] = profile;
        } else {
            servers.push(profile);
        }
        self.save_all(&servers);
    }

    pub fn delete_server(&self, id: &str) {
        let mut servers = self.get_all();
        servers.retain(|s| s.id != id);
        self.save_all(&servers);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_store_load() {
        let store = ServerStore::new();
        let servers = store.get_all();
        println!("Loaded {} servers from config", servers.len());
        for s in &servers {
            println!("Server: {} ({}:{}) proto={:?}", s.name, s.host, s.port, s.proto);
        }
    }
}
