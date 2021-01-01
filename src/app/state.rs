use serde::Serialize;
use std::collections::HashMap;

#[derive(Clone, Serialize)]
pub struct ChannelInfo {
    pub channel: String,
    pub virt_channels: u32,
    pub voltage_range: f32,
}

#[derive(Clone, Serialize)]
pub struct DeviceInfo {
    pub pico_scope_type: String,
    pub channel_info: Vec<ChannelInfo>,
    pub refresh_rate: u32,
}

impl ToString for DeviceInfo {
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

pub struct AppState {
    pub voltage: HashMap<String,Vec<(f32, u128)>>,
    pub device_info: DeviceInfo
}

impl AppState {
    pub fn new(device_info: DeviceInfo) -> Self {
        AppState {
            voltage: HashMap::new(),
            device_info
        }
    }
}