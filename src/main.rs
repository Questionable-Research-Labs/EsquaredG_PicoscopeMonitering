#![forbid(unsafe_code)]

pub mod app;
pub mod pico;

use actix_web::{middleware, web, App, HttpServer};
use anyhow::Result;
use console::{style, Term};
use dialoguer::Select;

use pico_sdk::prelude::*;

use crate::{
    app::{
        state::{AppState, DeviceInfo},
        *,
    },
    pico::*,
};

use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use crate::app::state::ChannelInfo;
use native_dialog::FileDialog;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

#[actix_web::main]
async fn main() -> Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=error,pico=error");
    env_logger::init();

    // Setup actix webserver
    let state = web::Data::new(Mutex::new(AppState::new(DeviceInfo {
        pico_scope_type: "".to_string(),
        channel_info: vec![],
        refresh_rate: 0,
    })));

    let state2 = state.clone();

    let web_server = HttpServer::new(move || {
        App::new()
            .service(index)
            .service(
                // All /api routes
                web::scope("/api")
                    .service(get_data)
                    .service(check_alive)
                    .service(device_info),
            )
            .service(actix_files::Files::new("/", "./static"))
            .app_data(state2.clone())
            .wrap(middleware::Logger::default())
    })
    .bind("127.0.0.1:8000")?;

    // Initlize picoscope
    let enumerator = DeviceEnumerator::with_resolution(cache_resolution());
    let device = select_device(&enumerator)?;
    let streaming_device = device.into_streaming_device();
    let ch_units = configure_channels(&streaming_device);

    let samples_per_second = get_capture_rate();

    // Initializing the state
    let mut locked_state = state.lock().unwrap();

    for channel in streaming_device.get_channels().iter() {
        locked_state.device_info.channel_info.push(ChannelInfo {
            channel: channel.to_string(),
            virt_channels: 1,
            voltage_range: 200f32,
        })
    }

    // locked_state.device_info.channel_info = channel_info;
    locked_state.device_info.pico_scope_type = streaming_device.get_variant();

    locked_state.device_info.refresh_rate = samples_per_second;

    drop(locked_state);

    // Start the webserver
    web_server.run();

    let instant = Instant::now();

    let capture_stats: Arc<dyn NewDataHandler> =
        CaptureStats::new(ch_units, state.clone(), instant);
    streaming_device.new_data.subscribe(capture_stats.clone());

    // let state3 = state.clone();

    let _sub = streaming_device.start(samples_per_second).unwrap();

    streaming_device.start(samples_per_second).unwrap();
    let terminal = Term::stdout();

    let cli_options = &[
        "Status",
        "Stop/Start Stream",
        "Save Data",
        "Clear memory",
        "Exit",
    ];
    let mut paused = false;

    let start_text = format!(
        "{} | {}",
        style("STARTED STREAMING").blue().bold(),
        style("Control panel at http://localhost:8000").green()
    );

    terminal
        .write_line(&format!(
            "\n{}\n{}\n{}\n",
            style("~".repeat(start_text.len())).blue(),
            start_text,
            style("~".repeat(start_text.len())).blue()
        ))
        .unwrap();
    loop {
        let cli_selection = Select::with_theme(&better_theme())
            .with_prompt(&format!(
                "{} {}",
                style(if paused { "Paused" } else { "Streaming!" })
                    .blue()
                    .underlined()
                    .bold(),
                style("Send a command in the console").green()
            ))
            .default(0)
            .items(cli_options)
            .interact()
            .unwrap();

        match cli_options[cli_selection] {
            "Status" => {
                print_stats(&state);
            }
            "Stop/Start Stream" => {
                if paused {
                    // Start Stream
                    terminal
                        .write_line(&format!("{}", style("Resuming").green()))
                        .unwrap();
                    streaming_device.start(samples_per_second).unwrap();
                    paused = false;
                } else {
                    streaming_device.stop();
                    paused = true;
                }
            }
            "Save Data" => write_data(state.clone()),
            "Clear Memory" => clear_memory(state.clone()),
            "Exit" => {
                streaming_device.stop();
                return Ok(());
            }

            _ => {
                println!("Unemplemented Selection!")
            }
        }
    }
}

fn clear_memory(state: web::Data<Mutex<AppState>>) {
    state.lock().unwrap().voltage_stream = HashMap::new();
}

fn write_data(state: web::Data<Mutex<AppState>>) {
    let cwd = std::env::current_dir().unwrap();
    let terminal = Term::stdout();

    let save_path = match match FileDialog::new()
        .set_location(&cwd)
        .add_filter("CSV File", &["csv"])
        .show_save_single_file()
    {
        Ok(a) => a,
        Err(err) => {
            terminal
                .write_line(&format!(
                    "{} {}{}\n        {:?}\n",
                    style("✘").bold().red(),
                    style("Error ").bold().red(),
                    style("could not display dialog").bold().green(),
                    err
                ))
                .unwrap();
            return ();
        }
    } {
        Some(a) => a,
        None => {
            terminal
                .write_line(&format!(
                    "{} {}{}\n",
                    style("✘").bold().red(),
                    style("Error ").bold().red(),
                    style("no file selected").bold().green(),
                ))
                .unwrap();
            return ();
        }
    };

    println!("{}", save_path.display());

    let mut file: File = match File::create(save_path) {
        Err(err) => {
            terminal
                .write_line(&format!(
                    "{} {}{}\n        {}\n",
                    style("✘").bold().red(),
                    style("Error ").bold().red(),
                    style("could not create file").bold().green(),
                    err
                ))
                .unwrap();
            return ();
        }
        Ok(a) => a,
    };

    let mut writer = csv::Writer::from_writer(vec![]);

    let state_locked = state.lock().unwrap();

    for (channel, voltages) in &state_locked.voltage_stream {
        for voltage in voltages {
            writer
                .write_record(&[
                    format!("{}", channel),
                    format!("{}", voltage.0),
                    format!("{}", voltage.1),
                ])
                .unwrap();
        }
    }

    let csv_data = String::from_utf8(writer.into_inner().unwrap()).unwrap();
    match file.write_all(csv_data.as_bytes()) {
        Err(err) => {
            terminal
                .write_line(&format!(
                    "{} {}{}\n        {}\n",
                    style("✘").bold().red(),
                    style("Error ").bold().red(),
                    style("could not write to file").bold().green(),
                    err
                ))
                .unwrap();
            return ();
        }
        Ok(_) => {}
    }
}
