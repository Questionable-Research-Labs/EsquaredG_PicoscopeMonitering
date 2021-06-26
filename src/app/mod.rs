pub mod state;

use actix_web::{get, web::Data, HttpResponse};
use parking_lot::Mutex;

use std::time::Instant;

use crate::pico::clear_and_get_memory;

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

    return HttpResponse::Ok()
        .content_type("application/json")
        .body(device_info.to_string());
}
