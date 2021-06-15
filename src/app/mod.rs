pub mod state;

use actix_web::{
    get, 
    HttpResponse,
    web::Data
};
use parking_lot::Mutex;

use std::{
    collections::{HashMap, VecDeque},
    time::Instant
};

use super::state::AppState;

use serde_json;



// /
#[get("/")]
pub fn index() -> HttpResponse {
    HttpResponse::Ok().body(std::fs::read_to_string("./static/index.html").unwrap())
}

// Mounts to /api
#[get("/")]
pub fn api_index() -> HttpResponse {
    let result = format!(r#"{{ "version": 0.1.0}}"#);

    HttpResponse::Ok().body(result)
}

// Mounts to /api/data
#[get("/data")]
pub fn get_data(state: Data<Mutex<AppState>>) -> HttpResponse {
    let start = Instant::now();
    let mut app_state = state.lock();
    let voltages: HashMap<String, VecDeque<(f64, String)>> = app_state.voltage_queue.clone().iter().map(|(channel,v)| 
        (channel.to_owned(),
            v.iter().map(|(voltage,time)|
            (voltage.to_owned(),time.to_string())
        ).collect())
    ).collect();
    let keys = app_state.voltage_queue.clone();
    for channel in keys.keys() {
        app_state.voltage_queue.get_mut(channel).unwrap().clear()
    }
    drop(app_state);


    let json_voltage = serde_json::to_string(&voltages).unwrap();

    let result = format!(r#"{{ "voltages": {} }}"#, json_voltage);
    println!("API lock time = {:?} ms",Instant::now()-start);
    return HttpResponse::Ok().content_type("application/json").body(result)
}

// Mounts to /api/alive
#[get("/alive")]
pub fn check_alive() -> HttpResponse {
    HttpResponse::Ok().body("Ya think")
}

// Mounts to /api/device-info
#[get("/device-info")]
pub fn device_info(state: Data<Mutex<AppState>>) -> HttpResponse {
    let locked_state = state.lock();
    let device_info = locked_state.device_info.clone();
    drop(locked_state);
    
    return HttpResponse::Ok().content_type("application/json").body(device_info.to_string());
}
