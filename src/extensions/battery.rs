use crate::config::AppConfig;
use super::api::{ExtensionMetadata, ExtensionResult, ExtensionListItem, ExtensionListAction, FlareExtension};
use std::fs;
use std::path::Path;

pub struct Battery;

impl Battery {
    fn get_battery_info(&self) -> Vec<ExtensionListItem> {
        let mut items = Vec::new();
        let base_path = Path::new("/sys/class/power_supply");

        if let Ok(entries) = fs::read_dir(base_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                
                if name.starts_with("BAT") {
                    let status = fs::read_to_string(path.join("status"))
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|_| "Unknown".to_string());
                    let capacity = fs::read_to_string(path.join("capacity"))
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|_| "0".to_string());
                    
                    let model = fs::read_to_string(path.join("model_name"))
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|_| name.to_string());

                    items.push(ExtensionListItem {
                        title: format!("{} ({}%)  {}", model, capacity, status),
                        value: String::new(),
                        action: Some(ExtensionListAction::None),
                    });

                    // Time to empty/full (if available)
                    if let Ok(energy_now) = self.read_sysfs_u64(&path.join("energy_now")) {
                        if let Ok(power_now) = self.read_sysfs_u64(&path.join("power_now")) {
                            if power_now > 0 {
                                if status == "Discharging" {
                                    let hours = energy_now as f64 / power_now as f64;
                                    let h = hours.floor() as u32;
                                    let m = ((hours % 1.0) * 60.0).round() as u32;
                                    items.push(ExtensionListItem {
                                        title: format!("Time Remaining   {}h {}m", h, m),
                                        value: String::new(),
                                        action: Some(ExtensionListAction::None),
                                    });
                                } else if status == "Charging" {
                                    if let Ok(energy_full) = self.read_sysfs_u64(&path.join("energy_full")) {
                                        if energy_full > energy_now {
                                            let hours = (energy_full - energy_now) as f64 / power_now as f64;
                                            let h = hours.floor() as u32;
                                            let m = ((hours % 1.0) * 60.0).round() as u32;
                                            items.push(ExtensionListItem {
                                                title: format!("Time to Full   {}h {}m", h, m),
                                                value: String::new(),
                                                action: Some(ExtensionListAction::None),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    } else if let Ok(charge_now) = self.read_sysfs_u64(&path.join("charge_now")) {
                        // Some laptops use charge/current instead of energy/power
                        if let Ok(current_now) = self.read_sysfs_u64(&path.join("current_now")) {
                             if current_now > 0 {
                                if status == "Discharging" {
                                    let hours = charge_now as f64 / current_now as f64;
                                    let h = hours.floor() as u32;
                                    let m = ((hours % 1.0) * 60.0).round() as u32;
                                    items.push(ExtensionListItem {
                                        title: format!("Time Remaining   {}h {}m", h, m),
                                        value: String::new(),
                                        action: Some(ExtensionListAction::None),
                                    });
                                } else if status == "Charging" {
                                    if let Ok(charge_full) = self.read_sysfs_u64(&path.join("charge_full")) {
                                        if charge_full > charge_now {
                                            let hours = (charge_full - charge_now) as f64 / current_now as f64;
                                            let h = hours.floor() as u32;
                                            let m = ((hours % 1.0) * 60.0).round() as u32;
                                            items.push(ExtensionListItem {
                                                title: format!("Time to Full   {}h {}m", h, m),
                                                value: String::new(),
                                                action: Some(ExtensionListAction::None),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Power consumption
                    if let Ok(power_now) = self.read_sysfs_u64(&path.join("power_now")) {
                        items.push(ExtensionListItem {
                            title: format!("Power Usage   {:.2} W", power_now as f64 / 1_000_000.0),
                            value: String::new(),
                            action: Some(ExtensionListAction::None),
                        });
                    }

                    // Cycle count
                    if let Ok(cycles) = fs::read_to_string(path.join("cycle_count")) {
                        let cycles = cycles.trim();
                        if !cycles.is_empty() {
                            items.push(ExtensionListItem {
                                title: format!("Cycle Count   {}", cycles),
                                value: String::new(),
                                action: Some(ExtensionListAction::None),
                            });
                        }
                    }
                }
            }
        }

        if items.is_empty() {
             items.push(ExtensionListItem {
                title: "No battery found".to_string(),
                value: "".to_string(),
                action: Some(ExtensionListAction::None),
            });
        }

        items
    }

    fn read_sysfs_u64(&self, path: &Path) -> Result<u64, ()> {
        fs::read_to_string(path)
            .map_err(|_| ())?
            .trim()
            .parse::<u64>()
            .map_err(|_| ())
    }
}

impl FlareExtension for Battery {
    fn metadata(&self, config: &AppConfig) -> ExtensionMetadata {
        ExtensionMetadata {
            name: "Battery".to_string(),
            description: "Show battery status and information".to_string(),
            trigger: config.features.battery_search_trigger.clone(),
            query_example: Some(config.features.battery_search_trigger.clone()),
        }
    }

    fn should_handle(&self, query: &str, config: &AppConfig) -> bool {
        query.starts_with(&config.features.battery_search_trigger)
    }

    fn process(&self, _query: &str, _config: &AppConfig, _registry: &crate::extensions::ExtensionRegistry) -> ExtensionResult {
        ExtensionResult::List {
            title: "Battery Info".to_string(),
            items: self.get_battery_info(),
            action: ExtensionListAction::None,
        }
    }
}
