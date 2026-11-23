#![allow(dead_code)]
use std::net::TcpStream;
use std::path::Path;
use ssh2::Session;
use suppaftp::{FtpStream, NativeTlsConnector, NativeTlsFtpStream};
use crate::models::{FileItem, Protocol};

pub enum ClientConnection {
    Ftp(FtpStream),
    Ftps(NativeTlsFtpStream),
    Sftp {
        session: Session,
        sftp: ssh2::Sftp,
    },
}

pub struct BackendEngine;

impl BackendEngine {
    pub fn sanitize_host(input: &str) -> (String, Option<u16>) {
        let mut clean = input.trim().to_string();
        if let Some(pos) = clean.find("://") {
            clean = clean[pos + 3..].to_string();
        }
        if let Some(pos) = clean.find('/') {
            clean = clean[..pos].to_string();
        }
        if let Some(pos) = clean.find('@') {
            clean = clean[pos + 1..].to_string();
        }
        if let Some(pos) = clean.rfind(':') {
            let port_str = &clean[pos + 1..];
            if let Ok(p) = port_str.parse::<u16>() {
                return (clean[..pos].to_string(), Some(p));
            }
        }
        (clean, None)
    }

    pub fn connect(
        proto: Protocol,
        host: &str,
        port: u16,
        user: &str,
        pass: &str,
    ) -> Result<ClientConnection, String> {
        let (clean_host, override_port) = Self::sanitize_host(host);
        let actual_port = override_port.unwrap_or(port);

        match proto {
            Protocol::Ftp => {
                let addr = format!("{}:{}", clean_host, actual_port);
                let mut ftp = FtpStream::connect(&addr).map_err(|e| format!("FTP Bağlantı Hatası: {}", e))?;
                ftp.login(user, pass).map_err(|e| format!("FTP Giriş Başarısız: {}", e))?;
                ftp.set_mode(suppaftp::Mode::Passive);
                Ok(ClientConnection::Ftp(ftp))
            }
            Protocol::Ftps => {
                let addr = format!("{}:{}", clean_host, actual_port);
                let ftp = NativeTlsFtpStream::connect(&addr).map_err(|e| format!("FTPS Bağlantı Hatası: {}", e))?;
                let connector = native_tls::TlsConnector::builder()
                    .danger_accept_invalid_certs(true)
                    .build()
                    .map_err(|e| format!("TLS Yapılandırma Hatası: {}", e))?;
                let mut ftps = ftp.into_secure(NativeTlsConnector::from(connector), &clean_host)
                    .map_err(|e| format!("TLS El Sıkışması Başarısız: {}", e))?;
                ftps.login(user, pass).map_err(|e| format!("FTPS Giriş Başarısız: {}", e))?;
                ftps.set_mode(suppaftp::Mode::Passive);
                Ok(ClientConnection::Ftps(ftps))
            }
            Protocol::Sftp => {
                let addr = format!("{}:{}", clean_host, actual_port);
                let tcp = TcpStream::connect(&addr).map_err(|e| format!("SSH/SFTP Bağlantı Hatası: {}", e))?;
                let mut sess = Session::new().map_err(|e| format!("SSH Oturumu Oluşturulamadı: {}", e))?;
                sess.set_tcp_stream(tcp);
                sess.handshake().map_err(|e| format!("SSH El Sıkışması Başarısız: {}", e))?;

                sess.userauth_password(user, pass)
                    .map_err(|e| format!("SSH Kimlik Doğrulama Başarısız: {}", e))?;
                if !sess.authenticated() {
                    return Err("Kimlik doğrulama reddedildi.".to_string());
                }

                let sftp = sess.sftp().map_err(|e| format!("SFTP Kanalı Açılamadı: {}", e))?;
                Ok(ClientConnection::Sftp { session: sess, sftp })
            }
        }
    }

    fn parse_ftp_list_lines(list_lines: Vec<String>, clean_path: &str) -> Vec<FileItem> {
        let mut items = Vec::new();
        for line in list_lines {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 9 {
                continue;
            }
            let perms = parts[0];
            let is_dir = perms.starts_with('d');
            let size = parts[4].parse::<u64>().unwrap_or(0);
            let name = parts[8..].join(" ");
            if name == "." || name == ".." {
                continue;
            }

            let full_path = if clean_path == "/" {
                format!("/{}", name)
            } else {
                format!("{}/{}", clean_path.trim_end_matches('/'), name)
            };

            items.push(FileItem {
                name,
                is_dir,
                size,
                modified_time: chrono::Utc::now().timestamp(),
                permissions: perms.to_string(),
                path: full_path,
            });
        }
        items
    }

    pub fn list_directory(conn: &mut ClientConnection, path: &str) -> Result<Vec<FileItem>, String> {
        let clean_path = if path.is_empty() { "/" } else { path };

        match conn {
            ClientConnection::Ftp(ftp) => {
                let _ = ftp.cwd(clean_path);
                let list_lines = ftp.list(None).map_err(|e| format!("FTP Listeleme Hatası: {}", e))?;
                Ok(Self::parse_ftp_list_lines(list_lines, clean_path))
            }
            ClientConnection::Ftps(ftps) => {
                let _ = ftps.cwd(clean_path);
                let list_lines = ftps.list(None).map_err(|e| format!("FTPS Listeleme Hatası: {}", e))?;
                Ok(Self::parse_ftp_list_lines(list_lines, clean_path))
            }
            ClientConnection::Sftp { sftp, .. } => {
                let p = Path::new(clean_path);
                let dir_entries = sftp.readdir(p).map_err(|e| format!("SFTP Listeleme Hatası: {}", e))?;
                let mut items = Vec::new();

                for (path_buf, stat) in dir_entries {
                    let name = path_buf.file_name().unwrap_or_default().to_string_lossy().to_string();
                    if name == "." || name == ".." {
                        continue;
                    }
                    let is_dir = stat.is_dir();
                    let size = stat.size.unwrap_or(0);
                    let full_path = path_buf.to_string_lossy().to_string();

                    items.push(FileItem {
                        name,
                        is_dir,
                        size,
                        modified_time: stat.mtime.unwrap_or(0) as i64,
                        permissions: format!("{:o}", stat.perm.unwrap_or(0)),
                        path: full_path,
                    });
                }
                Ok(items)
            }
        }
    }

    pub fn create_directory(conn: &mut ClientConnection, current_dir: &str, folder_name: &str) -> Result<(), String> {
        match conn {
            ClientConnection::Ftp(ftp) => {
                let _ = ftp.cwd(current_dir);
                ftp.mkdir(folder_name).map_err(|e| format!("Klasör oluşturulamadı: {}", e))?;
                Ok(())
            }
            ClientConnection::Ftps(ftps) => {
                let _ = ftps.cwd(current_dir);
                ftps.mkdir(folder_name).map_err(|e| format!("Klasör oluşturulamadı: {}", e))?;
                Ok(())
            }
            ClientConnection::Sftp { sftp, .. } => {
                let full = if current_dir == "/" {
                    format!("/{}", folder_name)
                } else {
                    format!("{}/{}", current_dir.trim_end_matches('/'), folder_name)
                };
                sftp.mkdir(Path::new(&full), 0o755).map_err(|e| format!("Klasör oluşturulamadı: {}", e))?;
                Ok(())
            }
        }
    }

    pub fn delete_item(conn: &mut ClientConnection, full_path: &str, is_dir: bool) -> Result<(), String> {
        let parent = Path::new(full_path).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "/".into());
        let name = Path::new(full_path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();

        match conn {
            ClientConnection::Ftp(ftp) => {
                let _ = ftp.cwd(&parent);
                if is_dir {
                    ftp.rmdir(&name).map_err(|e| format!("Klasör silinemedi: {}", e))?;
                } else {
                    ftp.rm(&name).map_err(|e| format!("Dosya silinemedi: {}", e))?;
                }
                Ok(())
            }
            ClientConnection::Ftps(ftps) => {
                let _ = ftps.cwd(&parent);
                if is_dir {
                    ftps.rmdir(&name).map_err(|e| format!("Klasör silinemedi: {}", e))?;
                } else {
                    ftps.rm(&name).map_err(|e| format!("Dosya silinemedi: {}", e))?;
                }
                Ok(())
            }
            ClientConnection::Sftp { sftp, .. } => {
                let p = Path::new(full_path);
                if is_dir {
                    sftp.rmdir(p).map_err(|e| format!("Klasör silinemedi: {}", e))?;
                } else {
                    sftp.unlink(p).map_err(|e| format!("Dosya silinemedi: {}", e))?;
                }
                Ok(())
            }
        }
    }

    pub fn rename_item(conn: &mut ClientConnection, old_path: &str, new_path: &str) -> Result<(), String> {
        let parent = Path::new(old_path).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "/".into());
        let old_name = Path::new(old_path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let new_name = Path::new(new_path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();

        match conn {
            ClientConnection::Ftp(ftp) => {
                let _ = ftp.cwd(&parent);
                ftp.rename(&old_name, &new_name).map_err(|e| format!("Yeniden adlandırılamadı: {}", e))?;
                Ok(())
            }
            ClientConnection::Ftps(ftps) => {
                let _ = ftps.cwd(&parent);
                ftps.rename(&old_name, &new_name).map_err(|e| format!("Yeniden adlandırılamadı: {}", e))?;
                Ok(())
            }
            ClientConnection::Sftp { sftp, .. } => {
                sftp.rename(Path::new(old_path), Path::new(new_path), None)
                    .map_err(|e| format!("Yeniden adlandırılamadı: {}", e))?;
                Ok(())
            }
        }
    }

    pub fn download_file(conn: &mut ClientConnection, remote_path: &str, local_path: &Path) -> Result<(), String> {
        let parent = Path::new(remote_path).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "/".into());
        let name = Path::new(remote_path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();

        match conn {
            ClientConnection::Ftp(ftp) => {
                let _ = ftp.cwd(&parent);
                let mut local_file = std::fs::File::create(local_path).map_err(|e| format!("Yerel dosya oluşturulamadı: {}", e))?;
                ftp.retr(&name, |stream| {
                    std::io::copy(stream, &mut local_file).map_err(|e| suppaftp::FtpError::ConnectionError(e))?;
                    Ok(())
                }).map_err(|e| format!("FTP İndirme Hatası: {}", e))?;
                Ok(())
            }
            ClientConnection::Ftps(ftps) => {
                let _ = ftps.cwd(&parent);
                let mut local_file = std::fs::File::create(local_path).map_err(|e| format!("Yerel dosya oluşturulamadı: {}", e))?;
                ftps.retr(&name, |stream| {
                    std::io::copy(stream, &mut local_file).map_err(|e| suppaftp::FtpError::ConnectionError(e))?;
                    Ok(())
                }).map_err(|e| format!("FTPS İndirme Hatası: {}", e))?;
                Ok(())
            }
            ClientConnection::Sftp { sftp, .. } => {
                let mut remote_file = sftp.open(Path::new(remote_path)).map_err(|e| format!("SFTP dosya açılamadı: {}", e))?;
                let mut local_file = std::fs::File::create(local_path).map_err(|e| format!("Yerel dosya oluşturulamadı: {}", e))?;
                std::io::copy(&mut remote_file, &mut local_file).map_err(|e| format!("SFTP İndirme Hatası: {}", e))?;
                Ok(())
            }
        }
    }

    pub fn upload_file(conn: &mut ClientConnection, local_path: &Path, remote_path: &str) -> Result<(), String> {
        let parent = Path::new(remote_path).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "/".into());
        let name = Path::new(remote_path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();

        match conn {
            ClientConnection::Ftp(ftp) => {
                let _ = ftp.cwd(&parent);
                let mut local_file = std::fs::File::open(local_path).map_err(|e| format!("Yerel dosya açılamadı: {}", e))?;
                ftp.put_file(&name, &mut local_file).map_err(|e| format!("FTP Yükleme Hatası: {}", e))?;
                Ok(())
            }
            ClientConnection::Ftps(ftps) => {
                let _ = ftps.cwd(&parent);
                let mut local_file = std::fs::File::open(local_path).map_err(|e| format!("Yerel dosya açılamadı: {}", e))?;
                ftps.put_file(&name, &mut local_file).map_err(|e| format!("FTPS Yükleme Hatası: {}", e))?;
                Ok(())
            }
            ClientConnection::Sftp { sftp, .. } => {
                let mut local_file = std::fs::File::open(local_path).map_err(|e| format!("Yerel dosya açılamadı: {}", e))?;
                let mut remote_file = sftp.create(Path::new(remote_path)).map_err(|e| format!("SFTP uzak dosya oluşturulamadı: {}", e))?;
                std::io::copy(&mut local_file, &mut remote_file).map_err(|e| format!("SFTP Yükleme Hatası: {}", e))?;
                Ok(())
            }
        }
    }
}
