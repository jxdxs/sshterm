use ssh2::{Session, Channel, Sftp};
use std::io::Read;
use std::net::TcpStream;
use std::path::Path;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfig {
    pub hostname: String,
    pub port: u16,
    pub username: String,
    pub auth_type: String,
    pub password: Option<String>,
    pub key_path: Option<String>,
    pub key_passphrase: Option<String>,
}

pub struct SshSession {
    session: Mutex<Session>,
    sftp: Mutex<Option<Sftp>>,
}

impl SshSession {
    pub fn connect(config: &SshConfig) -> Result<Self, String> {
        let addr = format!("{}:{}", config.hostname, config.port);
        let tcp = TcpStream::connect(&addr)
            .map_err(|e| format!("TCP connect failed: {}", e))?;
        tcp.set_read_timeout(Some(std::time::Duration::from_secs(30)))
            .map_err(|e| format!("Set timeout failed: {}", e))?;
        tcp.set_write_timeout(Some(std::time::Duration::from_secs(30)))
            .map_err(|e| format!("Set timeout failed: {}", e))?;

        let mut session = Session::new()
            .map_err(|e| format!("Session create failed: {}", e))?;
        session.set_tcp_stream(tcp);
        session.handshake()
            .map_err(|e| format!("SSH handshake failed: {}", e))?;

        session.set_banner("").ok();

        match config.auth_type.as_str() {
            "password" => {
                let password = config.password.as_deref().unwrap_or("");
                session.userauth_password(&config.username, password)
                    .map_err(|e| format!("Password auth failed: {}", e))?;
            }
            "key" => {
                let key_path = config.key_path.as_deref().unwrap_or("");
                let passphrase = config.key_passphrase.as_deref();
                session.userauth_pubkey_file(&config.username, None, Path::new(key_path), passphrase)
                    .map_err(|e| format!("Key auth failed: {}", e))?;
            }
            _ => return Err(format!("Unknown auth type: {}", config.auth_type)),
        }

        if !session.authenticated() {
            return Err("Authentication failed".to_string());
        }

        Ok(SshSession {
            session: Mutex::new(session),
            sftp: Mutex::new(None),
        })
    }

    pub fn create_shell(&self, cols: u32, rows: u32) -> Result<Channel, String> {
        let session = self.session.lock().map_err(|e| e.to_string())?;
        let mut channel = session.channel_session()
            .map_err(|e| format!("Channel open failed: {}", e))?;
        channel.request_pty_size(cols, rows, None, None)
            .map_err(|e| format!("PTY resize failed: {}", e))?;
        channel.shell()
            .map_err(|e| format!("Shell start failed: {}", e))?;
        Ok(channel)
    }

    pub fn exec_command(&self, command: &str) -> Result<String, String> {
        let session = self.session.lock().map_err(|e| e.to_string())?;
        let mut channel = session.channel_session()
            .map_err(|e| format!("Channel open failed: {}", e))?;
        channel.exec(command)
            .map_err(|e| format!("Exec failed: {}", e))?;

        let mut output = String::new();
        channel.read_to_string(&mut output)
            .map_err(|e| format!("Read failed: {}", e))?;
        channel.wait_close()
            .map_err(|e| format!("Wait close failed: {}", e))?;
        Ok(output)
    }

    pub fn get_sftp(&self) -> Result<Sftp, String> {
        let mut sftp_guard = self.sftp.lock().map_err(|e| e.to_string())?;
        let session = self.session.lock().map_err(|e| e.to_string())?;
        let sftp = session.sftp()
            .map_err(|e| format!("SFTP init failed: {}", e))?;
        *sftp_guard = Some(sftp);
        Err("SFTP session created but must be accessed via dedicated methods".to_string())
    }

    pub fn disconnect(&self) -> Result<(), String> {
        let session = self.session.lock().map_err(|e| e.to_string())?;
                session.disconnect(None::<ssh2::DisconnectCode>, "bye", None::<&str>)
            .map_err(|e| format!("Disconnect failed: {}", e))?;
        Ok(())
    }
}

// ─── SFTP 操作 ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub size: i64,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub permissions: String,
    pub modified: String,
}

pub struct SftpSession {
    sftp: Sftp,
}

impl SftpSession {
    pub fn new(sftp: Sftp) -> Self {
        SftpSession { sftp }
    }

    pub fn list_dir(&self, path: &str) -> Result<Vec<FileEntry>, String> {
        let entries = self.sftp.readdir(Path::new(path))
            .map_err(|e| format!("Read dir failed: {}", e))?;

        let mut files = Vec::new();
        for (entry_path, stat) in entries {
            let name = entry_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            if name.starts_with('.') { continue; } // 隐藏文件

            files.push(FileEntry {
                name,
                path: entry_path.to_string_lossy().to_string(),
                size: stat.size.unwrap_or(0) as i64,
                is_dir: stat.is_dir(),
                is_symlink: stat.file_type().is_symlink(),
                permissions: format!("{:o}", stat.perm.unwrap_or(0)),
                modified: stat.mtime
                    .map(|t| {
                        use chrono::DateTime;
                        let dt = DateTime::from_timestamp(t as i64, 0)
                            .unwrap_or_default();
                        dt.format("%Y-%m-%d %H:%M:%S").to_string()
                    })
                    .unwrap_or_default(),
            });
        }

        // 文件夹在前，名称排序
        files.sort_by(|a, b| {
            if a.is_dir != b.is_dir {
                b.is_dir.cmp(&a.is_dir)
            } else {
                a.name.cmp(&b.name)
            }
        });

        Ok(files)
    }

    pub fn read_file(&self, path: &str) -> Result<Vec<u8>, String> {
        let mut file = self.sftp.open(Path::new(path))
            .map_err(|e| format!("Open file failed: {}", e))?;
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut file, &mut data)
            .map_err(|e| format!("Read file failed: {}", e))?;
        Ok(data)
    }

    pub fn write_file(&self, path: &str, data: &[u8]) -> Result<(), String> {
        use std::io::Write;
        let mut file = self.sftp.create(Path::new(path))
            .map_err(|e| format!("Create file failed: {}", e))?;
        file.write_all(data)
            .map_err(|e| format!("Write file failed: {}", e))?;
        Ok(())
    }

    pub fn remove_file(&self, path: &str) -> Result<(), String> {
        self.sftp.unlink(Path::new(path))
            .map_err(|e| format!("Remove file failed: {}", e))
    }

    pub fn remove_dir(&self, path: &str) -> Result<(), String> {
        self.sftp.rmdir(Path::new(path))
            .map_err(|e| format!("Remove dir failed: {}", e))
    }

    pub fn create_dir(&self, path: &str) -> Result<(), String> {
        self.sftp.mkdir(Path::new(path), 0o755)
            .map_err(|e| format!("Create dir failed: {}", e))
    }

    pub fn rename(&self, from: &str, to: &str) -> Result<(), String> {
        self.sftp.rename(Path::new(from), Path::new(to), None)
            .map_err(|e| format!("Rename failed: {}", e))
    }

    pub fn stat(&self, path: &str) -> Result<FileEntry, String> {
        let stat = self.sftp.stat(Path::new(path))
            .map_err(|e| format!("Stat failed: {}", e))?;
        let name = Path::new(path).file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        Ok(FileEntry {
            name,
            path: path.to_string(),
            size: stat.size.unwrap_or(0) as i64,
            is_dir: stat.is_dir(),
            is_symlink: stat.file_type().is_symlink(),
            permissions: format!("{:o}", stat.perm.unwrap_or(0)),
            modified: stat.mtime
                .map(|t| {
                    let dt = chrono::DateTime::from_timestamp(t as i64, 0)
                        .unwrap_or_default();
                    dt.format("%Y-%m-%d %H:%M:%S").to_string()
                })
                .unwrap_or_default(),
        })
    }
}
