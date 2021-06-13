use serde::Serialize;

use std::{collections::{HashMap,VecDeque},convert::TryFrom};
use console::style;
use signifix::metric;


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
    pub voltage_stream: HashMap<String,Vec<(f64, u128)>>,
    pub voltage_queue: HashMap<String,VecDeque<(f64, u128)>>,
    pub device_info: DeviceInfo,
    pub streaming_speed: u64,
}

impl AppState {
    pub fn new(device_info: DeviceInfo) -> Self {
        AppState {
            voltage_stream: HashMap::new(),
            voltage_queue: HashMap::new(),
            device_info,
            streaming_speed: 0u64
        }
    }

}