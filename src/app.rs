use rocket::*;
use rocket::response::content::Json;

use std::collections::{HashMap};

use pico_sdk::{
    common::{PicoChannel, PicoRange},
    device::{ChannelDetails, PicoDevice},
    download::{cache_resolution, download_drivers_to_cache},
    enumeration::{DeviceEnumerator, EnumerationError},
    streaming::{StreamingEvent, SubscribeToReader, ToStreamDevice},
};

pub struct App {
    pub voltage: u64,
	pub voltage_units: String,
	pub channel_configuration: HashMap<PicoChannel, String>,
}

impl App {
    pub fn new(voltage_units: String, channel_configuration: HashMap<PicoChannel, String>) -> Self {    
        App {
            voltage: 0,
			voltage_units,
			channel_configuration
        }
	}
}


#[get("/data")]
pub fn get_data(state: State<App>) -> Json<String> {
    let result = format!(
		r#"{{
        value: {},
        voltage_units: {}
		}}"#,
		state.voltage.to_string(), state.voltage_units.to_owned());
    return Json(result)
}