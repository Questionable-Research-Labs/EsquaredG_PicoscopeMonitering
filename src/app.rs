use actix_web::{get, HttpResponse};
use std::sync::Mutex;

use serde_json;

use pico_sdk::{
    common::{PicoChannel},
};
use std::collections::HashMap;

pub struct AppState {
    pub voltage: Mutex<Vec<f64>>,
    pub voltage_units: Mutex<String>,
    pub channel_configuration: Mutex<HashMap<PicoChannel, String>>,
}

impl AppState {
    pub fn new<T>(voltage_units: &T, channel_configuration: HashMap<PicoChannel, String>) -> Self where T: ToString + ?Sized{
        AppState {
            voltage: Mutex::new(vec![0.0]),
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
    let mut voltage = app_state.voltage.lock().unwrap();
    let units = app_state.voltage_units.lock().unwrap();

    let json_voltage = match serde_json::to_string(&voltage.iter().map(|f| f.to_owned()).collect::<Vec<f64>>()) {
        Ok(voltage) => voltage,
        Err(error) => {
            return HttpResponse::InternalServerError().body(
                format!(
                    r#"{{ "code":{}, "overview": {}, "error":{:?} }}"#,
                    500,"Internal error serializing data",error
                ))
        }
    };

    let result = format!(r#"{{ "voltage": {}, "units": "{}" }}"#, json_voltage, units);

    *voltage = vec!();

    return HttpResponse::Ok().body(result)
}
