#![forbid(unsafe_code)]
pub mod app;
pub mod pico;

use anyhow::Result;
use console::Term;
use actix_web::{middleware, web, App, HttpServer};


use pico_sdk::{
    download::cache_resolution,
    enumeration::DeviceEnumerator,
    streaming::{SubscribeToReader, ToStreamDevice},
};

use crate::{
    app::{
        *,
        state::{
            AppState, DeviceInfo
        }
    },
    pico::*
};

use std::{
    sync::Mutex,
    time::Instant,
};

use log::info;

#[actix_web::main]
async fn main() -> Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info,pico=info");
    env_logger::init();

    // Setup actix webserver
    let state = web::Data::new(Mutex::new(AppState::new(DeviceInfo {
        pico_scope_type: "".to_string(),
        channel_count: 1,
        virt_channel_count: 0,
        refresh_rate: 0,
        voltage_range: 0,
        time_running: 0
    })));

    let state2 = state.clone();

    let web_server = HttpServer::new(move || {
        App::new()
            .service(index)
            .service(
                // All /api routes
                web::scope("/api")
                    .service(get_data)
                    .service(api_index)
                    .service(check_alive)
                    .service(device_info),
            )
            .service(actix_files::Files::new("/", "./static"))
            .app_data(state2.clone())
            .wrap(middleware::Logger::default())
    })
    .bind("127.0.0.1:8000")?;

    let enumerator = DeviceEnumerator::with_resolution(cache_resolution());
    let device = select_device(&enumerator)?;
    let ch_units = configure_channels(&device);

    let mut locked_state = state.lock().unwrap();

    let mut enabled_channels = vec!();

    for (channel, details) in device.channels.read().iter() {
        if details.configuration.enabled {
            enabled_channels.push((channel.to_owned(), details.to_owned()));
        }
    }

    locked_state.device_info.channel_count = enabled_channels.len() as u32;
    locked_state.device_info.pico_scope_type = (&device.variant).to_owned();

    let streaming_device = device.to_streaming_device();
    let capture_rate = get_capture_rate();

    locked_state.device_info.refresh_rate = (&capture_rate).to_owned();

    drop(locked_state);

    let term = Term::stdout();
    let rate_calc = RateCalc::new();

    web_server.run();
    let mut instant = Instant::now();

    let _sub = streaming_device
        .events
        .subscribe_on_thread(Box::new(move |event| {
            display_capture_stats(
                event,
                &term,
                &rate_calc,
                &ch_units,
                state.clone(),
                &mut instant,
            );
        }));

    println!("Press Enter to stop streaming");

    streaming_device.start(capture_rate).unwrap();

    Term::stdout().read_line().unwrap();

    streaming_device.stop();

    Ok(())
}
