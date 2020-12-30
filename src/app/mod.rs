pub mod state;

use actix_web::{
    get, 
    HttpResponse,
    web::Data
};

use std::{
    sync::Mutex
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
    let locked_state = state.lock().unwrap();
    let device_info = locked_state.device_info.clone();
    drop(locked_state);
    
    return HttpResponse::Ok().content_type("application/json").body(device_info.to_string());
}
