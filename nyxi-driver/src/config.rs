use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct SupportDev {
    #[serde(default)]
    pub vid: Option<u32>,
    #[serde(default)]
    pub pid: Option<u32>,
    #[serde(default)]
    pub usage: Option<u32>,
    #[serde(default)]
    pub usage_page: Option<u32>,
    #[serde(default)]
    pub interface_number: Option<i32>,
    #[serde(default)]
    pub dongle_common: bool,
    #[serde(default)]
    pub ble: bool,
    #[serde(default)]
    pub report_id: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SupportConfig {
    #[serde(default)]
    pub support: Vec<SupportDev>,
    #[serde(default)]
    pub boot: Vec<SupportDev>,
    #[serde(default)]
    pub vender: Vec<SupportDev>,
}

impl SupportConfig {
    pub fn load_or_default() -> Self {
        // Look in current dir, parent dir, or next to exe
        let candidates = [
            Path::new("supported_devices.toml"),
            Path::new("nyxi-driver/supported_devices.toml"),
            Path::new("../supported_devices.toml"),
        ];

        for path in &candidates {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(path) {
                    if let Ok(cfg) = toml::from_str::<SupportConfig>(&content) {
                        tracing::info!("Loaded device config from {:?}", path);
                        return cfg;
                    }
                }
            }
        }

        // Embedded fallback from supported_devices.toml
        const EMBEDDED_TOML: &str = include_str!("../supported_devices.toml");
        match toml::from_str::<SupportConfig>(EMBEDDED_TOML) {
            Ok(cfg) => {
                tracing::info!("Loaded embedded device configuration ({} devices)", cfg.support.len());
                cfg
            }
            Err(e) => {
                tracing::warn!("Failed to parse embedded config: {}", e);
                SupportConfig::default()
            }
        }
    }

    pub fn is_supported(&self, vid: u16, pid: u16, usage: u16, usage_page: u16, interface: i32) -> Option<&SupportDev> {
        self.support.iter().find(|d| {
            let vid_match = d.vid.map(|v| v as u16 == vid).unwrap_or(true);
            let pid_match = d.pid.map(|p| p as u16 == pid).unwrap_or(true);
            let usage_match = d.usage.map(|u| u as u16 == usage).unwrap_or(true);
            let up_match = d.usage_page.map(|up| up as u16 == usage_page).unwrap_or(true);
            let if_match = d.interface_number.map(|i| i == -1 || i == interface).unwrap_or(true);

            vid_match && pid_match && usage_match && up_match && if_match
        })
    }
}
