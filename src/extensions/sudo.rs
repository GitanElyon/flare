use crate::config::AppConfig;
use super::api::{AuthResult, ExtensionMetadata};
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct ParsedSudoQuery {
    pub query: String,
    pub sudo_args: Vec<String>,
}

pub struct Sudo;

impl crate::extensions::FlareExtension for Sudo {
    fn metadata(&self, config: &AppConfig) -> crate::extensions::ExtensionMetadata {
        metadata(config)
    }

    fn should_handle(&self, query: &str, _config: &AppConfig) -> bool {
        query.starts_with("sudo")
    }

    fn process(&self, _query: &str, _config: &AppConfig, _registry: &crate::extensions::ExtensionRegistry) -> crate::extensions::ExtensionResult {
        // Sudo intercepts launch via requires_auth/authenticate_and_launch
        crate::extensions::ExtensionResult::None
    }

    fn strip_prefix(&self, query: &str, _config: &AppConfig) -> Option<(String, Vec<String>)> {
        if !query.starts_with("sudo") {
            return None;
        }
        let parsed = parse_query(query);
        Some((parsed.query, parsed.sudo_args))
    }

    fn requires_auth(&self, query: &str, _config: &AppConfig) -> bool {
        query.starts_with("sudo")
    }

    fn authenticate_and_launch(
        &self,
        password: &str,
        cmd: &str,
        args: &[String],
        prefix_args: &[String],
    ) -> AuthResult {
        // Validate password
        let validation_args: Vec<String> = prefix_args
            .iter()
            .filter(|arg| {
                ["-u", "-g", "-h", "-p", "-n", "-k", "-S"].contains(&arg.as_str())
                    || !arg.starts_with('-')
            })
            .cloned()
            .collect();

        let child = Command::new("sudo")
            .args(&validation_args)
            .arg("-v")
            .arg("-S")
            .arg("-k")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        match child {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() {
                    if writeln!(stdin, "{}", password).is_err() {
                        return AuthResult::LaunchError("Failed to write password".to_string());
                    }
                }
                match child.wait() {
                    Ok(status) => {
                        if !status.success() {
                            return AuthResult::AuthFailed;
                        }
                        // Launch with sudo
                        let mut command = Command::new("sudo");
                        command.args(prefix_args);
                        command.arg("-b");
                        command.arg("-S");
                        command.arg(cmd);
                        command.args(args);
                        command
                            .stdin(Stdio::piped())
                            .stdout(Stdio::null())
                            .stderr(Stdio::null());
                        unsafe {
                            command.pre_exec(|| {
                                libc::setsid();
                                libc::signal(libc::SIGHUP, libc::SIG_IGN);
                                Ok(()) as std::io::Result<()>
                            });
                        }
                        match command.spawn() {
                            Ok(mut child) => {
                                if let Some(mut stdin) = child.stdin.take() {
                                    let _ = writeln!(stdin, "{}", password);
                                }
                                AuthResult::Success
                            }
                            Err(e) => AuthResult::LaunchError(format!("Failed to launch: {}", e)),
                        }
                    }
                    Err(e) => AuthResult::LaunchError(format!("sudo check failed: {}", e)),
                }
            }
            Err(e) => AuthResult::LaunchError(format!("Failed to run sudo: {}", e)),
        }
    }
}

pub fn metadata(_config: &AppConfig) -> crate::extensions::ExtensionMetadata {
    ExtensionMetadata {
        name: "Sudo".to_string(),
        description: "Run commands with sudo privileges".to_string(),
        trigger: "sudo".to_string(),
        query_example: Some("sudo ".to_string()),
    }
}

pub fn parse_query(search_query: &str) -> ParsedSudoQuery {
    if !search_query.starts_with("sudo") {
        return ParsedSudoQuery {
            query: search_query.to_string(),
            sudo_args: Vec::new(),
        };
    }

    let parts: Vec<&str> = search_query.split_whitespace().collect();
    let mut idx = 1usize;
    let mut sudo_args = Vec::new();

    if parts.first() == Some(&"sudo") {
        while idx < parts.len() {
            let part = parts[idx];
            if part.starts_with('-') {
                sudo_args.push(part.to_string());
                if ["-C", "-g", "-h", "-p", "-r", "-t", "-U", "-u"].contains(&part) {
                    if idx + 1 < parts.len() {
                        idx += 1;
                        sudo_args.push(parts[idx].to_string());
                    }
                }
            } else {
                break;
            }
            idx += 1;
        }
    }

    let query = if idx < parts.len() {
        parts[idx..].join(" ")
    } else {
        String::new()
    };

    ParsedSudoQuery {
        query,
        sudo_args,
    }
}
