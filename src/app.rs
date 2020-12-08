use actix_web::{get, HttpResponse};
use std::sync::Mutex;

use pico_sdk::{
    common::{PicoChannel},
};
use std::collections::HashMap;

pub struct AppState {
    pub voltage: Mutex<f64>,
    pub voltage_units: Mutex<String>,
    pub channel_configuration: Mutex<HashMap<PicoChannel, String>>,
}

impl AppState {
    pub fn new<T>(voltage_units: &T, channel_configuration: HashMap<PicoChannel, String>) -> Self where T: ToString + ?Sized{
        AppState {
            voltage: Mutex::new(0.0),
            voltage_units: Mutex::new(voltage_units.to_string()),
            channel_configuration: Mutex::new(channel_configuration),
        }
    }
}

#[get("/")]
pub fn index(state: actix_web::web::Data<Mutex<AppState>>) -> HttpResponse {
    let result = format!(r#"{{ "version": 0.1.0}}"#);

    HttpResponse::Ok().body(result)
}

#[get("/data")]
pub fn get_data(state: actix_web::web::Data<Mutex<AppState>>) -> HttpResponse {
    let app_state = state.lock().unwrap();
    let voltage = app_state.voltage_units.lock().unwrap();
    let units = app_state.voltage_units.lock().unwrap();

    let result = format!(r#"{{ "voltage": {}, "units": "{}" }}"#, voltage, units);

    HttpResponse::Ok().body(result)
}
