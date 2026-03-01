use crate::config::AppConfig;
use super::api::{ExtensionListAction, ExtensionListItem, ExtensionMetadata, ExtensionResult, FlareExtension};
use std::process::{Command, Stdio};

pub struct Volume;

// ─── Backend ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Backend {
    /// PipeWire — controlled via `wpctl`
    Wpctl,
    /// PulseAudio — controlled via `pactl`
    Pactl,
    /// ALSA — controlled via `amixer`
    Amixer,
}

fn command_exists(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn detect_backend() -> Option<Backend> {
    if command_exists("wpctl") { return Some(Backend::Wpctl); }
    if command_exists("pactl")  { return Some(Backend::Pactl);  }
    if command_exists("amixer") { return Some(Backend::Amixer); }
    None
}

// ─── Volume queries ───────────────────────────────────────────────────────────

/// Returns `(volume_percent, is_muted)`.
fn get_volume_info() -> (Option<u32>, Option<bool>) {
    // wpctl (PipeWire)
    if command_exists("wpctl") {
        if let Ok(out) = Command::new("wpctl")
            .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
            .output()
        {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout);
                let muted = s.contains("[MUTED]");
                if let Some(rest) = s.trim().strip_prefix("Volume:") {
                    let num_str = rest.split_whitespace().next().unwrap_or("").trim();
                    if let Ok(v) = num_str.parse::<f64>() {
                        return (Some((v * 100.0).round() as u32), Some(muted));
                    }
                }
            }
        }
    }

    // pactl (PulseAudio)
    if command_exists("pactl") {
        let volume = Command::new("pactl")
            .args(["get-sink-volume", "@DEFAULT_SINK@"])
            .output()
            .ok()
            .and_then(|out| {
                if !out.status.success() { return None; }
                let s = String::from_utf8_lossy(&out.stdout).to_string();
                s.split('/').find_map(|part| {
                    let p = part.trim();
                    if p.ends_with('%') {
                        p.trim_end_matches('%').trim().parse::<u32>().ok()
                    } else {
                        None
                    }
                })
            });
        let muted = Command::new("pactl")
            .args(["get-sink-mute", "@DEFAULT_SINK@"])
            .output()
            .ok()
            .and_then(|out| {
                if out.status.success() {
                    Some(String::from_utf8_lossy(&out.stdout).to_lowercase().contains("yes"))
                } else {
                    None
                }
            });
        if volume.is_some() || muted.is_some() {
            return (volume, muted);
        }
    }

    // amixer (ALSA)
    if command_exists("amixer") {
        if let Ok(out) = Command::new("amixer").args(["get", "Master"]).output() {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).to_string();
                let volume = s.lines().find_map(|line| {
                    if line.contains('[') && line.contains('%') {
                        let start = line.find('[')? + 1;
                        let end = line.find('%')?;
                        line[start..end].parse::<u32>().ok()
                    } else {
                        None
                    }
                });
                let muted = s.lines().any(|line| line.contains("[off]"));
                if volume.is_some() {
                    return (volume, Some(muted));
                }
            }
        }
    }

    (None, None)
}

// ─── Command builders — all backends ─────────────────────────────────────────

fn set_volume_cmd(backend: &Backend, percent: u32) -> String {
    match backend {
        Backend::Wpctl  => format!("wpctl set-volume @DEFAULT_AUDIO_SINK@ {}%", percent),
        Backend::Pactl  => format!("pactl set-sink-volume @DEFAULT_SINK@ {}%", percent),
        Backend::Amixer => format!("amixer set Master {}%", percent),
    }
}

fn adjust_volume_cmd(backend: &Backend, delta: i32) -> String {
    match backend {
        Backend::Wpctl => {
            if delta >= 0 {
                format!("wpctl set-volume -l 1.5 @DEFAULT_AUDIO_SINK@ {}%+", delta)
            } else {
                format!("wpctl set-volume @DEFAULT_AUDIO_SINK@ {}%-", delta.unsigned_abs())
            }
        }
        Backend::Pactl => {
            if delta >= 0 {
                format!("pactl set-sink-volume @DEFAULT_SINK@ +{}%", delta)
            } else {
                format!("pactl set-sink-volume @DEFAULT_SINK@ -{}%", delta.unsigned_abs())
            }
        }
        Backend::Amixer => {
            if delta >= 0 {
                format!("amixer set Master {}%+", delta)
            } else {
                format!("amixer set Master {}%-", delta.unsigned_abs())
            }
        }
    }
}

fn mute_toggle_cmd(backend: &Backend) -> String {
    match backend {
        Backend::Wpctl  => "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle".to_string(),
        Backend::Pactl  => "pactl set-sink-mute @DEFAULT_SINK@ toggle".to_string(),
        Backend::Amixer => "amixer set Master toggle".to_string(),
    }
}

// ─── Device listing — wpctl and pactl ─────────────────────────────────────────

/// `(id_or_name, display_label)` for each output device.
fn list_sinks(backend: &Backend) -> Vec<(String, String)> {
    match backend {
        Backend::Wpctl => {
            // Parse `wpctl status` — look for the "Sinks:" section under "Audio"
            // Each sink line: "    * 47. Built-in Audio Stereo   [vol: 0.50]"
            //              or: "      48. HDMI / DisplayPort      [vol: 0.40]"
            let Ok(out) = Command::new("wpctl").arg("status").output() else {
                return Vec::new();
            };
            if !out.status.success() { return Vec::new(); }
            let text = String::from_utf8_lossy(&out.stdout);
            let mut in_audio = false;
            let mut in_sinks = false;
            let mut sinks = Vec::new();
            for line in text.lines() {
                if line.trim_start().starts_with("Audio") {
                    in_audio = true;
                    continue;
                }
                if in_audio && line.contains("Sinks:") {
                    in_sinks = true;
                    continue;
                }
                if in_sinks {
                    // A blank line or a new section header ends the sinks block
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        in_sinks = false;
                        in_audio = false;
                        continue;
                    }
                    // Line format: "    * 47. Name   [vol: ...]" or "      47. Name   [vol: ...]"
                    let stripped = trimmed.trim_start_matches('*').trim();
                    if let Some(dot_pos) = stripped.find('.') {
                        let id = stripped[..dot_pos].trim().to_string();
                        let rest = stripped[dot_pos + 1..].trim();
                        // Remove trailing [vol: ...] annotation
                        let name = rest.split('[').next().unwrap_or(rest).trim().to_string();
                        if !id.is_empty() && !name.is_empty() {
                            // id here is the numeric node ID used by `wpctl set-default`
                            sinks.push((id, name));
                        }
                    }
                }
            }
            sinks
        }
        Backend::Pactl => {
            let Ok(out) = Command::new("pactl").args(["list", "short", "sinks"]).output() else {
                return Vec::new();
            };
            if !out.status.success() { return Vec::new(); }
            // Format: <id>\t<name>\t<module>\t<sample_spec>\t<state>
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|line| {
                    let cols: Vec<&str> = line.splitn(5, '\t').collect();
                    if cols.len() >= 2 {
                        Some((cols[1].to_string(), cols[1].to_string()))
                    } else {
                        None
                    }
                })
                .collect()
        }
        Backend::Amixer => Vec::new(),
    }
}

fn set_default_sink_cmd(backend: &Backend, id_or_name: &str) -> String {
    match backend {
        Backend::Wpctl  => format!("wpctl set-default {}", id_or_name),
        Backend::Pactl  => format!("pactl set-default-sink {}", id_or_name),
        Backend::Amixer => String::new(),
    }
}

// ─── Display helpers ──────────────────────────────────────────────────────────

/// Renders an ASCII block bar that fills `width` characters.
/// Width is intentionally kept short (passed as a constant) so it fits inside
/// a list item without overflowing the UI box.
fn volume_bar(vol: u32, width: usize) -> String {
    let capped = vol.min(100);
    let filled = ((capped as f32 / 100.0) * width as f32).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("[{}{}]", "\u{2588}".repeat(filled), "\u{2591}".repeat(empty))
}

fn status_line(vol: Option<u32>, muted: Option<bool>) -> String {
    match (vol, muted) {
        (Some(v), Some(true)) => format!("  {}% {} [MUTED]", v, volume_bar(v, 20)),
        (Some(v), _)          => format!("  {}% {}", v, volume_bar(v, 20)),
        _                     => "  Volume status unavailable".to_string(),
    }
}

// ─── Extension impl ───────────────────────────────────────────────────────────

impl FlareExtension for Volume {
    fn metadata(&self, _config: &AppConfig) -> ExtensionMetadata {
        ExtensionMetadata {
            name: "Volume".to_string(),
            description: "Control system audio volume (v! -h for help)".to_string(),
            trigger: "v!".to_string(),
            query_example: Some("v!".to_string()),
        }
    }

    fn should_handle(&self, query: &str, _config: &AppConfig) -> bool {
        query.starts_with("v!")
    }

    fn process(
        &self,
        query: &str,
        _config: &AppConfig,
        _registry: &crate::extensions::ExtensionRegistry,
    ) -> ExtensionResult {
        let Some(backend) = detect_backend() else {
            return ExtensionResult::List {
                title: " Volume ".to_string(),
                items: vec![ExtensionListItem { action: None,
                    title: "  No audio backend found (requires wpctl, pactl, or amixer)".to_string(),
                    value: String::new(),
                }],
                action: ExtensionListAction::None,
            };
        };

        let arg = query.strip_prefix("v!").unwrap_or("").trim();

        // ── Help ──────────────────────────────────────────────────────────────
        if arg == "-h" || arg == "--help" || arg == "help" {
            return ExtensionResult::List {
                title: " Volume Help ".to_string(),
                items: vec![
                    ExtensionListItem { action: None, title: "  v!           Open volume menu".to_string(),                       value: String::new() },
                    ExtensionListItem { action: None, title: "  v! -h        Show this help".to_string(),                         value: String::new() },
                    ExtensionListItem { action: None, title: "  v! +N        Increase volume by N%   (e.g. v! +10)".to_string(), value: String::new() },
                    ExtensionListItem { action: None, title: "  v! -N        Decrease volume by N%   (e.g. v! -5)".to_string(),  value: String::new() },
                    ExtensionListItem { action: None, title: "  v! N         Set volume to N%        (e.g. v! 75)".to_string(),  value: String::new() },
                    ExtensionListItem { action: None, title: "  v! mute      Toggle mute".to_string(),                            value: String::new() },
                    ExtensionListItem { action: None, title: "  v! devices   List and switch output devices".to_string(),         value: String::new() },
                ],
                action: ExtensionListAction::None,
            };
        }

        let (current_vol, muted) = get_volume_info();
        let muted_flag = muted.unwrap_or(false);

        // Title: just volume percentage and mute state — no bar here to avoid overflow
        let title = match (current_vol, muted_flag) {
            (Some(v), true)  => format!(" Volume {}% [MUTED] ", v),
            (Some(v), false) => format!(" Volume {}% ", v),
            _                => " Volume ".to_string(),
        };

        // ── Mute toggle ───────────────────────────────────────────────────────
        if arg == "mute" || arg == "m" {
            let label = if muted_flag { "  Unmute" } else { "  Mute" };
            return ExtensionResult::List {
                title,
                items: vec![ExtensionListItem { action: None,
                    title: label.to_string(),
                    value: mute_toggle_cmd(&backend),
                }],
                action: ExtensionListAction::ExecuteAndRefresh,
            };
        }

        // ── Device list ───────────────────────────────────────────────────────
        if arg == "devices" || arg == "d" {
            let sinks = list_sinks(&backend);
            if sinks.is_empty() {
                return ExtensionResult::List {
                    title: " Volume - Output Devices ".to_string(),
                    items: vec![ExtensionListItem { action: None,
                        title: "  No output devices found".to_string(),
                        value: String::new(),
                    }],
                    action: ExtensionListAction::None,
                };
            }
            let items = sinks
                .into_iter()
                .map(|(id, name)| {
                    let cmd = set_default_sink_cmd(&backend, &id);
                    ExtensionListItem { action: None, title: format!("  {}", name), value: cmd }
                })
                .filter(|item| !item.value.is_empty())
                .collect();
            return ExtensionResult::List {
                title: " Volume - Output Devices ".to_string(),
                items,
                action: ExtensionListAction::ExecuteAndRefresh,
            };
        }

        // ── Increase +N ───────────────────────────────────────────────────────
        if let Some(rest) = arg.strip_prefix('+') {
            if let Ok(delta) = rest.parse::<u32>() {
                return ExtensionResult::List {
                    title,
                    items: vec![ExtensionListItem { action: None,
                        title: format!("  Increase volume by {}%", delta),
                        value: adjust_volume_cmd(&backend, delta as i32),
                    }],
                    action: ExtensionListAction::ExecuteAndRefresh,
                };
            }
        }

        // ── Decrease -N ───────────────────────────────────────────────────────
        // Note: "-N" must come after "-h" check above so "v! -h" isn't caught here.
        if let Some(rest) = arg.strip_prefix('-') {
            if let Ok(delta) = rest.parse::<u32>() {
                return ExtensionResult::List {
                    title,
                    items: vec![ExtensionListItem { action: None,
                        title: format!("  Decrease volume by {}%", delta),
                        value: adjust_volume_cmd(&backend, -(delta as i32)),
                    }],
                    action: ExtensionListAction::ExecuteAndRefresh,
                };
            }
        }

        // ── Set to N% ─────────────────────────────────────────────────────────
        if let Ok(target) = arg.parse::<u32>() {
            let clamped = target.min(150);
            return ExtensionResult::List {
                title,
                items: vec![ExtensionListItem { action: None,
                    title: format!("  Set volume to {}%", clamped),
                    value: set_volume_cmd(&backend, clamped),
                }],
                action: ExtensionListAction::ExecuteAndRefresh,
            };
        }

        // ── Default menu ──────────────────────────────────────────────────────
        let mute_label = if muted_flag { "  Unmute" } else { "  Mute" };
        let items = vec![
            // Status line at top — bar lives here (list item), not in the title
            ExtensionListItem { action: None,
                title: status_line(current_vol, muted),
                value: String::new(),
            },
            ExtensionListItem { action: None, title: "  Volume Up  (+5%)".to_string(),  value: adjust_volume_cmd(&backend, 5)   },
            ExtensionListItem { action: None, title: "  Volume Down (-5%)".to_string(), value: adjust_volume_cmd(&backend, -5)  },
            ExtensionListItem { action: None, title: mute_label.to_string(),            value: mute_toggle_cmd(&backend)         },
            ExtensionListItem { action: None, title: "  Set to 25%".to_string(),        value: set_volume_cmd(&backend, 25)      },
            ExtensionListItem { action: None, title: "  Set to 50%".to_string(),        value: set_volume_cmd(&backend, 50)      },
            ExtensionListItem { action: None, title: "  Set to 75%".to_string(),        value: set_volume_cmd(&backend, 75)      },
            ExtensionListItem { action: None, title: "  Set to 100%".to_string(),       value: set_volume_cmd(&backend, 100)     },
            ExtensionListItem { action: None, title: "  Output Devices (v! devices)".to_string(), value: String::new()           },
        ];

        ExtensionResult::List {
            title,
            items,
            action: ExtensionListAction::ExecuteAndRefresh,
        }
    }
}
