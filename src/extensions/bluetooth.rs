use crate::config::AppConfig;
use super::api::{ExtensionListAction, ExtensionListItem, ExtensionMetadata, ExtensionResult, FlareExtension};
use std::process::{Command, Stdio};

pub struct Bluetooth;

fn command_exists(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn _bool_to_status(val: bool) -> &'static str {
    if val { "ON" } else { "OFF" }
}

fn is_powered() -> bool {
    let Ok(out) = Command::new("bluetoothctl").arg("show").output() else {
        return false;
    };
    if !out.status.success() {
        return false;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    stdout.lines().any(|line| line.trim() == "Powered: yes")
}

fn is_scanning() -> bool {
    let Ok(out) = Command::new("bluetoothctl").arg("show").output() else {
        return false;
    };
    if !out.status.success() {
        return false;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    stdout.lines().any(|line| line.trim() == "Discovering: yes")
}

#[derive(Debug, Clone)]
struct BluetoothDevice {
    mac: String,
    name: String,
}

fn get_devices() -> Vec<BluetoothDevice> {
    let Ok(out) = Command::new("bluetoothctl").arg("devices").output() else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut devices = Vec::new();
    
    for line in stdout.lines() {
        // Output format: "Device XX:XX:XX:XX:XX:XX Device Name Here"
        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        if parts.len() >= 3 && parts[0] == "Device" {
            devices.push(BluetoothDevice {
                mac: parts[1].to_string(),
                name: parts[2].trim().to_string(),
            });
        }
    }
    
    devices
}

#[derive(Debug, Clone, Default)]
struct DeviceInfo {
    connected: bool,
    paired: bool,
    trusted: bool,
}

fn get_device_info(mac: &str) -> Option<DeviceInfo> {
    let Ok(out) = Command::new("bluetoothctl").args(["info", mac]).output() else {
        return None;
    };
    if !out.status.success() {
        return None;
    }
    
    let mut info = DeviceInfo::default();
    let stdout = String::from_utf8_lossy(&out.stdout);
    
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed == "Connected: yes" {
            info.connected = true;
        } else if trimmed == "Paired: yes" {
            info.paired = true;
        } else if trimmed == "Trusted: yes" {
            info.trusted = true;
        }
    }
    
    Some(info)
}

fn is_mac_address(s: &str) -> bool {
    let parts: Vec<&str> = s.split(':').collect();
    parts.len() == 6 && parts.iter().all(|p| p.len() == 2 && p.chars().all(|c| c.is_ascii_hexdigit()))
}

impl FlareExtension for Bluetooth {
    fn metadata(&self, _config: &AppConfig) -> ExtensionMetadata {
        ExtensionMetadata {
            name: "Bluetooth".to_string(),
            description: "Manage Bluetooth connections (b! -h for help)".to_string(),
            trigger: "b!".to_string(),
            query_example: Some("b!".to_string()),
        }
    }

    fn should_handle(&self, query: &str, _config: &AppConfig) -> bool {
        query.starts_with("b!")
    }

    fn process(
        &self,
        query: &str,
        _config: &AppConfig,
        _registry: &crate::extensions::ExtensionRegistry,
    ) -> ExtensionResult {
        if !command_exists("bluetoothctl") {
            return ExtensionResult::List {
                title: " Bluetooth ".to_string(),
                items: vec![ExtensionListItem { action: None,
                    title: "  `bluetoothctl` not found. Please install bluez/bluetoothctl.".to_string(),
                    value: String::new(),
                }],
                action: ExtensionListAction::None,
            };
        }

        let arg = query.strip_prefix("b!").unwrap_or("").trim();

        // ── Help Menu ─────────────────────────────────────────────────────────
        if arg == "-h" || arg == "--help" || arg == "help" {
            return ExtensionResult::List {
                title: " Bluetooth Help ".to_string(),
                items: vec![
                    ExtensionListItem { action: None, title: "  b!           Open main bluetooth menu".to_string(),     value: String::new() },
                    ExtensionListItem { action: None, title: "  b! power on  Turn on Bluetooth adapter".to_string(),    value: String::new() },
                    ExtensionListItem { action: None, title: "  b! power off Turn off Bluetooth adapter".to_string(),   value: String::new() },
                    ExtensionListItem { action: None, title: "  b! scan on   Start scanning for devices".to_string(),   value: String::new() },
                    ExtensionListItem { action: None, title: "  b! scan off  Stop scanning".to_string(),                value: String::new() },
                ],
                action: ExtensionListAction::None,
            };
        }

        // ── Direct Commands ───────────────────────────────────────────────────
        if arg == "power on" {
            return ExtensionResult::List {
                title: " Bluetooth Admin ".to_string(),
                items: vec![ExtensionListItem { action: None,
                    title: "  Powering on...".to_string(),
                    value: "bluetoothctl power on".to_string(),
                }],
                action: ExtensionListAction::ExecuteAndRefresh,
            };
        } else if arg == "power off" {
            return ExtensionResult::List {
                title: " Bluetooth Admin ".to_string(),
                items: vec![ExtensionListItem { action: None,
                    title: "  Powering off...".to_string(),
                    value: "bluetoothctl power off".to_string(),
                }],
                action: ExtensionListAction::ExecuteAndRefresh,
            };
        } else if arg == "scan on" {
            return ExtensionResult::List {
                title: " Bluetooth Admin ".to_string(),
                items: vec![ExtensionListItem { action: None,
                    title: "  Starting scan...".to_string(),
                    value: "bluetoothctl scan on".to_string(),
                }],
                action: ExtensionListAction::ExecuteAndRefresh,
            };
        } else if arg == "scan off" {
            return ExtensionResult::List {
                title: " Bluetooth Admin ".to_string(),
                items: vec![ExtensionListItem { action: None,
                    title: "  Stopping scan...".to_string(),
                    value: "bluetoothctl scan off".to_string(),
                }],
                action: ExtensionListAction::ExecuteAndRefresh,
            };
        }

        // ── Device detail menu ────────────────────────────────────────────────
        if is_mac_address(arg) {
            let mac = arg;
            let info = get_device_info(mac).unwrap_or_default();
            
            let status = if info.connected {
                "Connected"
            } else if info.paired {
                "Paired, Disconnected"
            } else {
                "Unpaired"
            };
            
            let mut items = vec![];
            
            // Go back
            items.push(ExtensionListItem {
                action: Some(ExtensionListAction::SetSearchQuery),
                title: "  <- Back to summary".to_string(),
                value: "b! ".to_string(),
            });
            
            items.push(ExtensionListItem { action: None,
                title: format!("  [{}]", status),
                value: String::new(),
            });

            if info.connected {
                items.push(ExtensionListItem { action: None,
                    title: "  Disconnect".to_string(),
                    value: format!("bluetoothctl disconnect {}", mac),
                });
            } else {
                items.push(ExtensionListItem { action: None,
                    title: "  Connect".to_string(),
                    value: format!("bluetoothctl connect {}", mac),
                });
            }

            if !info.paired {
                items.push(ExtensionListItem { action: None,
                    title: "  Pair".to_string(),
                    value: format!("bluetoothctl pair {}", mac),
                });
            }
            
            if !info.trusted {
                items.push(ExtensionListItem { action: None,
                    title: "  Trust (Auto-connect in future)".to_string(),
                    value: format!("bluetoothctl trust {}", mac),
                });
            } else {
                items.push(ExtensionListItem { action: None,
                    title: "  Untrust".to_string(),
                    value: format!("bluetoothctl untrust {}", mac),
                });
            }

            items.push(ExtensionListItem { action: None,
                title: "  Remove / Forget".to_string(),
                value: format!("bluetoothctl remove {}", mac),
            });

            return ExtensionResult::List {
                title: format!(" Bluetooth: {} ", mac),
                items,
                // If they click back, it does SetSearchQuery. If they click connect, it ExecuteAndRefresh
                // Since this uses different target actions, we need a smart way. 
                // Ah, wait! The list can only have ONE action type per list in the current API.
                // Wait! `ExtensionResult::List { action, ... }` defines the action for ALL items.
                // If we want both `SetSearchQuery` and `ExecuteAndRefresh` in the same list, we can't!
                action: ExtensionListAction::ExecuteAndRefresh, 
            };
        }

        // ── Main Menu ─────────────────────────────────────────────────────────
        let power = is_powered();
        let power_status_text = if power { "ON" } else { "OFF" };
        let scanning = is_scanning();
        
        let power_label = if power { "  Disable Bluetooth" } else { "  Enable Bluetooth" };

        let scan_label = if scanning { "  Stop Scanning" } else { "  Scan for Devices" };

        let mut items = vec![];
        
        // These can't be nicely mixed if action targets SetSearchQuery vs ExecuteAndRefresh.
        // Wait, NO! The API only allows one `ExtensionListAction` for the entire list!
        // To fix this without breaking the API, what if I make ALL items `SetSearchQuery`?
        // E.g. "Disable Bluetooth" sets search query to "b! power off" ?
        // That is brilliant!
        // So clicking "Enable Bluetooth" sets query to "b! power on", which immediately runs because of the Direct Commands check!
        
        items.push(ExtensionListItem { action: None,
            title: format!("  Power: [{}] -> {}", power_status_text, power_label.trim()),
            value: if power { "b! power off".to_string() } else { "b! power on".to_string() },
        });

        items.push(ExtensionListItem { action: None,
            title: format!("  Scan:  [{}] -> {}", if scanning { "ON" } else { "OFF" }, scan_label.trim()),
            value: if scanning { "b! scan off".to_string() } else { "b! scan on".to_string() },
        });
        
        items.push(ExtensionListItem { action: None,
            title: "  ────────────────────────────".to_string(),
            value: String::new(),
        });

        let devices = get_devices();
        if devices.is_empty() {
            items.push(ExtensionListItem { action: None,
                title: "  No devices found (Scan to find more)".to_string(),
                value: String::new(),
            });
        } else {
            for d in devices {
                items.push(ExtensionListItem { action: None,
                    title: format!("  {}  ({})", d.name, d.mac),
                    value: format!("b! {}", d.mac), // this sets the query!
                });
            }
        }

        ExtensionResult::List {
            title: " Bluetooth ".to_string(),
            items,
            action: ExtensionListAction::SetSearchQuery,
        }
    }
}
