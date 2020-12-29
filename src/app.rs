use actix_web::{get, HttpResponse};
use std::sync::Mutex;

use std::io::prelude::*;

use serde_json;

use pico_sdk::{
    common::{PicoChannel},
};
use std::collections::HashMap;

pub struct AppState {
    pub voltage: Vec<(f32, u128)>,
    pub channel_configuration: HashMap<PicoChannel, String>,
    pub start_time: std::time::Instant,
}

impl AppState {
    pub fn new(channel_configuration: HashMap<PicoChannel, String>) -> Self{
        AppState {
            voltage: vec!(),
            channel_configuration: channel_configuration,
            start_time: std::time::Instant::now(),
        }
    }
}

#[get("/")]
pub fn index() -> HttpResponse {
    HttpResponse::Ok().body(std::fs::read_to_string("./static/index.html").unwrap())
}

#[get("/")]
pub fn api_index(state: actix_web::web::Data<Mutex<AppState>>) -> HttpResponse {
    let result = format!(r#"{{ "version": 0.1.0}}"#);

    HttpResponse::Ok().body(result)
}

// Mounts to /api/data
#[get("/data")]
pub fn get_data(state: actix_web::web::Data<Mutex<AppState>>) -> HttpResponse {
    let mut app_state = state.lock().unwrap();
    let voltage = app_state.voltage.clone();
    app_state.voltage.drain(0..voltage.len());
    drop(app_state);

    let json_voltage = match serde_json::to_string(&voltage.iter().map(|f| f.to_owned()).collect::<Vec<(f32,u128)>>()) {
        Ok(voltage) => voltage,
        Err(error) => {
            return HttpResponse::InternalServerError().body(
                format!(
                    r#"{{ "code":{}, "overview": {}, "error":{:?} }}"#,
                    500,"Internal error serializing data",error
                ))
        }
    };

    let result = format!(r#"{{ "voltages": {} }}"#, json_voltage);

    

    // *app_state.voltage = 

    return HttpResponse::Ok().body(result)
}
