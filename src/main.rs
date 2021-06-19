#![forbid(unsafe_code)]

pub mod app;
pub mod pico;

use actix_web::{middleware, web, App, HttpServer};
use anyhow::Result;
use console::{style, Term};
use dialoguer::{Select,Input};
use chrono::prelude::{DateTime, Local};

use pico_sdk::prelude::*;

use crate::{
    app::{
        state::{AppState, DeviceInfo},
        *,
    },
    pico::*,
};

use parking_lot::Mutex;
use std::{fmt::{format, write}, sync::Arc, time::Instant};

use crate::app::state::ChannelInfo;
use native_dialog::FileDialog;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

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

    // Initialize picoscope
    let enumerator = DeviceEnumerator::with_resolution(cache_resolution());
    let device = select_device(&enumerator)?;
    let streaming_device = device.into_streaming_device();
    let ch_units = configure_channels(&streaming_device);

    let samples_per_second = get_capture_rate();

    // Initializing the state
    let mut locked_state = state.lock();

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

    let mut instant = Instant::now();

    let capture_stats: Arc<dyn NewDataHandler> =
        CaptureStats::new(ch_units, state.clone());
    streaming_device.new_data.subscribe(capture_stats.clone());

    // let state3 = state.clone();

    let _sub = streaming_device.start(samples_per_second).unwrap();

    streaming_device.start(samples_per_second).unwrap();
    let terminal = Term::stdout();

    let cli_options = &[
        "Status",
        "Stop/Start Recording",
        "Save Data without stopping",
        "Clear memory",
        "Exit",
    ];
    let mut recording = false;

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
        // let _ = io::stdin().read(&mut [0u8]).unwrap();

        let cli_selection = Select::with_theme(&better_theme())
            .with_prompt(&format!(
                "{} {}",
                style(if recording { "Paused" } else { "Streaming!" })
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
            "Stop/Start Recording" => {
                if recording {
                    // Start Stream
                    terminal
                        .write_line(&format!("{}", style("Resuming").green()))
                        .unwrap();
                    recording = false;
                    let mut instant = Instant::now();
                } else {
                    let cli_selection = Input::with_theme(&better_theme()).default(String::from("untitled_run")).interact().unwrap();
                    write_data(state.clone(),Some(format!("{}_{}",Local::now().format("%F_%T"),cli_selection)));
                    recording = true;
                }
            }
            "Save Data without stopping" => write_data(state.clone(), None),
            "Clear Memory" => clear_memory(state.clone()),
            "Exit" => {
                streaming_device.stop();
                return Ok(());
            }

            _ => {
                println!("Unimplemented Selection!")
            }
        }
    }
}

fn clear_memory(state: web::Data<Mutex<AppState>>) {
    state.lock().voltage_stream = HashMap::new();
}

fn write_data(state: web::Data<Mutex<AppState>>, defaults: Option<String>) {
    let cwd = std::env::current_dir().unwrap();
    let terminal = Term::stdout();
    let save_path;
    
    if !defaults.is_none() {
        save_path = cwd.join(format!("data_output/{}.csv",defaults.unwrap()));
    } else {
        save_path = match match FileDialog::new()
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
    }
    

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

    let state_locked = state.lock();

    for (channel, voltages) in state_locked.voltage_stream.clone().into_iter() {
        for voltage in voltages {
            writer
                .write_record(&[
                    format!("{}", channel),
                    format!("{}", voltage)
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
