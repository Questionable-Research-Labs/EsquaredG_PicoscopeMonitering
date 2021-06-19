#![forbid(unsafe_code)]

pub mod app;
pub mod pico;

use actix_web::{middleware, web, App, HttpServer};
use anyhow::Result;
use chrono::prelude::Local;
use console::{style, Term};
use dialoguer::{Input, Select};

use pico_sdk::prelude::*;

use crate::{
    app::{
        state::{AppState, DeviceInfo},
        *,
    },
    pico::*,
};

use parking_lot::Mutex;
use std::{
    sync::Arc
};

use crate::app::state::ChannelInfo;
use native_dialog::FileDialog;
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
    let mut recording_cache = locked_state.recording.clone();

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

    let capture_stats: Arc<dyn NewDataHandler> = CaptureStats::new(ch_units, state.clone());
    streaming_device.new_data.subscribe(capture_stats.clone());

    // let state3 = state.clone();

    let _sub = streaming_device.start(samples_per_second).unwrap();

    streaming_device.start(samples_per_second).unwrap();
    let terminal = Term::stdout();

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

        let cli_options = &[
            "Status",
            if recording_cache {
                "Stop Recording"
            } else {
                "Start Recording"
            },
            "Save Data without stopping",
            "Clear Memory",
            "Exit",
        ];

        let cli_selection = Select::with_theme(&better_theme())
            .with_prompt(&format!(
                "{} {}",
                style(if recording_cache {
                    style("Recording!").green()
                } else {
                    style("Not Recording").red()
                })
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
            "Stop Recording" => {
                let cli_selection = Input::with_theme(&better_theme())
                    .default(String::from("untitled_run"))
                    .interact()
                    .unwrap();
                write_data(
                    state.clone(),
                    Some(format!(
                        "{}_{}",
                        Local::now().format("%F_%T"),
                        cli_selection
                    )),
                );
                recording_cache = false;
                let mut unlocked_state = state.lock();
                unlocked_state.recording = false;
                drop(unlocked_state);
            }
            "Start Recording" => {
                // Start Stream
                terminal
                    .write_line(&format!("{}", style("Resuming").green()))
                    .unwrap();
                recording_cache = true;
                let mut unlocked_state = state.lock();
                unlocked_state.recording = true;
                drop(unlocked_state);
            }
            "Save Data without stopping" => write_data(state.clone(), None),
            "Clear Memory" => { let _ = clear_and_get_memory(state.clone(),true);},
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



fn write_data(state: web::Data<Mutex<AppState>>, defaults: Option<String>) {
    let cwd = std::env::current_dir().unwrap();
    let terminal = Term::stdout();
    let save_path;

    if !defaults.is_none() {
        save_path = cwd.join(format!("data_output/{}.csv", defaults.unwrap()));
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

    println!("Saving to: {}", save_path.display());
    terminal
        .write_line(&format!("{} {}", "⌛", style("Saving... ").bold(),))
        .unwrap();

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
    writer
        .write_record(&[format!("channel"), format!("voltage")])
        .unwrap();

    for (channel, voltages) in state_locked.voltage_stream.clone().into_iter() {
        for voltage in voltages {
            writer
                .write_record(&[format!("{}", channel), format!("{}", voltage)])
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
