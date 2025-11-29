use dirs::config_dir;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders},
};
use serde::{Deserialize, Serialize};
use std::fs;

pub struct ConfigLoadResult {
    pub config: AppConfig,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub features: FeaturesConfig,
    pub window: SectionConfig,
    pub outer_box: SectionConfig,
    pub input: SectionConfig,
    pub scroll: SectionConfig,
    pub inner_box: SectionConfig,
    pub entry: SectionConfig,
    pub entry_selected: SectionConfig,
    pub text: TextConfig,
}

impl AppConfig {
    pub fn load() -> ConfigLoadResult {
        let default = Self::default();
        let mut warning = None;
        let config = match config_dir() {
            Some(mut dir) => {
                dir.push("flare");
                if fs::create_dir_all(&dir).is_err() {
                    warning = Some("Unable to create ~/.config/flare, using defaults".into());
                    default
                } else {
                    let config_path = dir.join("config.toml");
                    if config_path.exists() {
                        match fs::read_to_string(&config_path) {
                            Ok(contents) => match toml::from_str::<AppConfig>(&contents) {
                                Ok(parsed) => parsed,
                                Err(err) => {
                                    warning = Some(format!(
                                        "Invalid config ({}). Falling back to defaults.",
                                        err
                                    ));
                                    default
                                }
                            },
                            Err(err) => {
                                warning = Some(format!(
                                    "Failed to read config ({}). Using defaults.",
                                    err
                                ));
                                default
                            }
                        }
                    } else {
                        if let Ok(serialized) = toml::to_string_pretty(&default) {
                            let _ = fs::write(&config_path, serialized);
                        }
                        default
                    }
                }
            }
            None => {
                warning = Some("Could not locate configuration directory. Using defaults.".into());
                default
            }
        };

        ConfigLoadResult { config, warning }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            features: FeaturesConfig::default(),
            window: SectionConfig {
                bg: Some(String::from("#000000")),
                ..SectionConfig::default()
            },
            outer_box: SectionConfig {
                title: Some(String::from(" Flare ")),
                border_color: Some(String::from("#cdd6f4")),
                ..SectionConfig::default()
            },
            input: SectionConfig {
                title: Some(String::from(" Search ")),
                border_color: Some(String::from("#cba6f7")),
                ..SectionConfig::default()
            },
            scroll: SectionConfig {
                border_color: Some(String::from("#585b70")),
                ..SectionConfig::default()
            },
            inner_box: SectionConfig {
                title: Some(String::from(" Applications ")),
                border_color: Some(String::from("#89b4fa")),
                ..SectionConfig::default()
            },
            entry: SectionConfig {
                bg: Some(String::from("#1e1e2e")),
                ..SectionConfig::default()
            },
            entry_selected: SectionConfig {
                fg: Some(String::from("#1e1e2e")),
                bg: Some(String::from("#cdd6f4")),
                ..SectionConfig::default()
            },
            text: TextConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct GeneralConfig {
    pub rounded_corners: bool,
    pub show_borders: bool,
    pub highlight_symbol: Option<String>,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            rounded_corners: true,
            show_borders: true,
            highlight_symbol: Some(String::from(">> ")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct FeaturesConfig {
    pub enable_file_explorer: bool,
    pub enable_launch_args: bool,
    pub enable_auto_complete: bool,
    pub dirs_first: bool,
}

impl Default for FeaturesConfig {
    fn default() -> Self {
        Self {
            enable_file_explorer: true,
            enable_launch_args: true,
            enable_auto_complete: true,
            dirs_first: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct SectionConfig {
    pub title: Option<String>,
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub border_color: Option<String>,
    pub rounded: Option<bool>,
    pub borders: Option<bool>,
    #[serde(alias = "visable")]
    pub visible: Option<bool>,
    pub title_alignment: Option<TextAlignment>,
}

impl SectionConfig {
    pub fn is_visible(&self) -> bool {
        self.visible.unwrap_or(true)
    }

    pub fn style(&self) -> Style {
        let mut style = Style::default();
        if let Some(color) = self.fg.as_deref().and_then(parse_color) {
            style = style.fg(color);
        }
        if let Some(color) = self.bg.as_deref().and_then(parse_color) {
            style = style.bg(color);
        }
        style
    }

    pub fn border_offset(&self, general: &GeneralConfig) -> u16 {
        if self.draws_borders(general) { 1 } else { 0 }
    }

    pub fn draws_borders(&self, general: &GeneralConfig) -> bool {
        self.borders.unwrap_or(general.show_borders)
    }

    pub fn block<'a>(&self, general: &GeneralConfig, fallback_title: &'a str) -> Block<'a> {
        let mut block = Block::default().title(
            self.title
                .clone()
                .unwrap_or_else(|| fallback_title.to_string()),
        );

        block = block.title_alignment(self.title_alignment.unwrap_or(TextAlignment::Left).into());

        if self.draws_borders(general) {
            block = block.borders(Borders::ALL);
            let rounded = self.rounded.unwrap_or(general.rounded_corners);
            block = block.border_type(if rounded {
                BorderType::Rounded
            } else {
                BorderType::Plain
            });

            if let Some(color) = self.border_color.as_deref().and_then(parse_color) {
                block = block.border_style(Style::default().fg(color));
            }
        }

        block.style(self.style())
    }
}

impl Default for SectionConfig {
    fn default() -> Self {
        Self {
            title: None,
            fg: None,
            bg: None,
            border_color: None,
            rounded: None,
            borders: None,
            visible: None,
            title_alignment: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct TextConfig {
    #[serde(flatten)]
    pub section: SectionConfig,
    pub alignment: Option<TextAlignment>,
}

impl TextConfig {
    pub fn style(&self) -> Style {
        self.section.style()
    }

    pub fn alignment(&self) -> TextAlignment {
        self.alignment.unwrap_or(TextAlignment::Left)
    }

    pub fn is_visible(&self) -> bool {
        self.section.is_visible()
    }
}

impl Default for TextConfig {
    fn default() -> Self {
        Self {
            section: SectionConfig {
                fg: Some(String::from("#f2f5f7")),
                ..SectionConfig::default()
            },
            alignment: Some(TextAlignment::Left),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

impl From<TextAlignment> for Alignment {
    fn from(value: TextAlignment) -> Self {
        match value {
            TextAlignment::Left => Alignment::Left,
            TextAlignment::Center => Alignment::Center,
            TextAlignment::Right => Alignment::Right,
        }
    }
}

pub fn parse_color(value: &str) -> Option<Color> {
    let trimmed = value.trim();
    if let Some(hex) = trimmed.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(Color::Rgb(r, g, b));
        } else if hex.len() == 8 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            let apply_alpha = |channel: u8| -> u8 {
                let value = (channel as u16 * a as u16) / 255;
                value as u8
            };
            return Some(Color::Rgb(apply_alpha(r), apply_alpha(g), apply_alpha(b)));
        }
    }

    match trimmed.to_ascii_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "white" => Some(Color::White),
        "gray" | "grey" => Some(Color::Gray),
        "darkgray" | "dark-grey" => Some(Color::DarkGray),
        "lightred" | "light-red" => Some(Color::LightRed),
        "lightgreen" | "light-green" => Some(Color::LightGreen),
        "lightblue" | "light-blue" => Some(Color::LightBlue),
        "lightmagenta" | "light-magenta" => Some(Color::LightMagenta),
        "lightcyan" | "light-cyan" => Some(Color::LightCyan),
        "lightyellow" | "light-yellow" => Some(Color::LightYellow),
        _ => None,
    }
}
