#![forbid(unsafe_code)]

pub mod app;
pub mod pico;

use anyhow::Result;
use console::{style, Term};
use actix_web::{middleware, web, App, HttpServer};
use dialoguer::Select;

use pico_sdk::{
    download::cache_resolution,
    enumeration::DeviceEnumerator,
    streaming::{SubscribeToReader, ToStreamDevice},
};

use crate::{
    app::{
        *,
        state::{
            AppState, DeviceInfo,
        },
    },
    pico::*,
};

use std::{
    sync::Mutex,
    time::Instant,
};

use crate::app::state::ChannelInfo;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::collections::HashMap;
use std::path::Path;
use native_dialog::FileDialog;

#[actix_web::main]
async fn main() -> Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=error,pico=debug");
    env_logger::init();

    // Setup actix webserver
    let state = web::Data::new(Mutex::new(AppState::new(DeviceInfo {
        pico_scope_type: "".to_string(),
        channel_info: vec!(),
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
    let ch_units = configure_channels(&device);

    let streaming_device = device.clone().to_streaming_device();
    let capture_rate = get_capture_rate();


    // Initializing the state
    let mut locked_state = state.lock().unwrap();


    for (channel, details) in device.channels.read().iter() {
        if details.configuration.enabled {
            locked_state.device_info.channel_info.push(ChannelInfo {
                channel: channel.to_string(),
                virt_channels: 1,
                voltage_range: details.configuration.range.get_max_scaled_value(),
            })
        }
    }

    // locked_state.device_info.channel_info = channel_info;
    locked_state.device_info.pico_scope_type = (&device.variant).to_owned();

    locked_state.device_info.refresh_rate = (&capture_rate).to_owned();

    drop(locked_state);

    // Start the webserver
    web_server.run();

    let mut instant = Instant::now();

    let state3 = state.clone();

    let _sub = streaming_device
        .events
        .subscribe_on_thread(Box::new(move |event| {
            voltage_capture_async(
                event,
                &ch_units,
                state3.clone(),
                &mut instant,
            );
        }));

    streaming_device.start(capture_rate).unwrap();
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
        style("Control pannel at http://localhost:8000").green()
    );

    terminal.write_line(&format!(
        "\n{}\n{}\n{}\n",
        style("~".repeat(start_text.len())).blue(),
        start_text,
        style("~".repeat(start_text.len())).blue()
    )).unwrap();
    loop {
        // terminal.write_line(&format!(
        //     "{} {}",
        //     style("Streaming!").blue().underlined().bold(),
        //     style("Press enter to stop streaming").green()
        // )).unwrap();
        // terminal.read_line().unwrap();
        // terminal.clear_last_lines(2)?;
        // streaming_device.stop();

        // terminal.write_line(&format!(
        //     "{} {}",
        //     style("Paused").blue().underlined().bold(),
        //     style("Press enter to start streaming").green()
        // )).unwrap();
        // terminal.read_line().unwrap();
        // terminal.clear_last_lines(2)?;
        // terminal.write_line(&format!(
        //     "{}",
        //     style("Starting").green()
        // )).unwrap();

        // streaming_device.start(capture_rate).unwrap();
        // terminal.clear_last_lines(1)?;
        let cli_selection = Select::with_theme(&better_theme())
            .with_prompt(&format!(
                "{} {}",
                style(if paused { "Paused" } else { "Streaming!" }).blue().underlined().bold(),
                style("Send a command in the console").green()
            ))
            .default(0)
            .items(cli_options)
            .interact()
            .unwrap();

        match cli_options[cli_selection] {
            "Status" => { println!("Stuff about Refresh rate, running modules and shit here") }
            "Stop/Start Stream" => {
                if paused {
                    // Start Stream
                    terminal.write_line(&format!(
                        "{}",
                        style("Resuming").green()
                    )).unwrap();
                    streaming_device.start(capture_rate).unwrap();
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

            _ => { println!("Unemplemented Selection!") }
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
        .show_save_single_file() {
        Ok(a) => a,
        Err(err) => {
            terminal.write_line(&format!("{} {}{}\n        {:?}\n", style("✘").bold().red(),
                                         style("Error ").bold().red(),
                                         style("could not display dialog").bold().green(),
                                         err)).unwrap();
            return ();
        }
    } {
        Some(a) => a,
        None => {
            terminal.write_line(&format!("{} {}{}\n", style("✘").bold().red(),
                                         style("Error ").bold().red(),
                                         style("no file selected").bold().green(),
            )).unwrap();
            return ();
        }
    };

    println!("{}", save_path.display());

    let mut file: File = match File::create(save_path) {
        Err(err) => {
            terminal.write_line(&format!("{} {}{}\n        {}\n", style("✘").bold().red(),
                                         style("Error ").bold().red(),
                                         style("could not create file").bold().green(),
                                         err)).unwrap();
            return ();
        }
        Ok(a) => a
    };

    let mut writer = csv::Writer::from_writer(vec![]);

    let state_locked = state.lock().unwrap();

    for (channel, voltages) in &state_locked.voltage_stream {
        for voltage in voltages {
            writer.write_record(&[format!("{}", channel), format!("{}", voltage.0), format!("{}", voltage.1)]).unwrap();
        }
    }

    let csv_data = String::from_utf8(writer.into_inner().unwrap()).unwrap();
    match file.write_all(csv_data.as_bytes()) {
        Err(err) => {
            terminal.write_line(&format!("{} {}{}\n        {}\n", style("✘").bold().red(),
                                         style("Error ").bold().red(),
                                         style("could not write to file").bold().green(),
                                         err)).unwrap();
            return ();
        }
        Ok(_) => {}
    }
}
