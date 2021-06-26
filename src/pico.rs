use crate::{
    app::state::AppState, virt_channels::split_into_virt_channels, write_data, ConstConfig,
};
use actix_web::web::{self, Data};
use anyhow::{anyhow, Result};
use chrono::Local;
use console::{style, Style};
use dialoguer::{theme::ColorfulTheme, Select};
use futures::lock;
use parking_lot::{Mutex, RawMutex};
use pico_sdk::prelude::*;
use procinfo::pid::statm_self;
use signifix::metric;
use tokio::pin;

use core::f64;
use std::{
    borrow::Borrow,
    collections::{HashMap, VecDeque},
    convert::TryFrom,
    iter::Iterator,
    sync::Arc,
    time::{Duration, Instant},
};

pub fn better_theme() -> ColorfulTheme {
    ColorfulTheme {
        defaults_style: Style::new(),
        inactive_item_style: Style::new(),
        active_item_style: Style::new().bold(),
        active_item_prefix: style(">".to_string()).for_stderr().bold().green(),
        ..ColorfulTheme::default()
    }
}

#[derive(Clone)]
struct RateCalc {
    queue: Arc<Mutex<VecDeque<(Instant, u64)>>>,
    window_size: Duration,
}

impl RateCalc {
    pub fn new(window_size: Duration) -> Self {
        RateCalc {
            queue: Default::default(),
            window_size,
        }
    }

    pub fn get_value(&self, latest: usize) -> u64 {
        let mut queue = self.queue.lock();
        queue.push_back((Instant::now(), latest as u64));

        let mut max = 0;
        let mut total = 0;
        for (index, (timestamp, value)) in queue.iter_mut().enumerate() {
            if timestamp.elapsed() > self.window_size {
                max = index;
            } else {
                total += *value;
            }
        }

        for _ in 0..max {
            queue.pop_front();
        }

        queue
            .front()
            .map(|(f, _)| (total as f64 / f.elapsed().as_secs_f64()) as u64)
            .unwrap_or(0)
    }
}

// pub fn main() -> Result<()> {
//     if std::env::args().any(|a| a.contains("--trace")) {
//         tracing_subscriber::fmt()
//             .with_max_level(tracing::Level::TRACE)
//             .with_span_events(tracing_subscriber::fmt::format::FmtSpan::ACTIVE)
//             .init();
//     }

//     let enumerator = DeviceEnumerator::with_resolution(cache_resolution());
//     let device = select_device(&enumerator)?;
//     let streaming_device = device.into_streaming_device();
//     let ch_units = configure_channels(&streaming_device);
//     let samples_per_second = get_capture_rate();
//     let capture_stats: Arc<dyn NewDataHandler> = CaptureStats::new(ch_units);
//     streaming_device.new_data.subscribe(capture_stats.clone());

//     println!("Press Enter to stop streaming");
//     streaming_device.start(samples_per_second).unwrap();

//     Term::stdout().read_line().unwrap();

//     streaming_device.stop();

//     Ok(())
// }

pub fn select_device(enumerator: &DeviceEnumerator) -> Result<PicoDevice> {
    loop {
        println!("Searching for devices...",);

        let devices = enumerator.enumerate();

        if devices.is_empty() {
            return Err(anyhow!("{}", style("No Pico devices found").red()));
        }

        let device_options = devices
            .iter()
            .map(|result| match result {
                Ok(d) => format!("PicoScope {} ({})", d.variant, d.serial),
                Err(EnumerationError::DriverLoadError { driver, .. }) => {
                    format!("PicoScope {} (Missing Driver)", driver)
                }
                Err(EnumerationError::DriverError { driver, error }) => {
                    format!("PicoScope {} (Driver Error - {})", driver, error)
                }
                Err(EnumerationError::KernelDriverError { driver }) => {
                    format!("PicoScope {} (Kernel Driver Missing)", driver)
                }
                Err(EnumerationError::VersionError { driver, .. }) => {
                    format!("PicoScope {} (Driver Version Error)", driver)
                }
            })
            .collect::<Vec<String>>();

        let device_selection = Select::with_theme(&better_theme())
            .with_prompt(&format!(
                "{}",
                style("Select a device").blue().underlined().bold()
            ))
            .default(0)
            .items(&device_options[..])
            .interact()
            .unwrap();

        println!();

        match &devices[device_selection] {
            Ok(d) => return Ok(d.open().unwrap()),
            Err(error) => match error {
                EnumerationError::DriverLoadError { driver, .. }
                | EnumerationError::VersionError {
                    driver,
                    found: _,
                    required: _,
                } => {
                    println!("Downloading {} driver", driver);
                    let _ = download_drivers_to_cache(&[*driver]);
                    println!("Download complete");
                }
                _ => {}
            },
        }
    }
}

pub fn configure_channels(device: &PicoStreamingDevice) -> HashMap<PicoChannel, String> {
    loop {
        let mut channels = device
            .get_channels()
            .iter()
            .map(|c| {
                (
                    *c,
                    device.get_valid_ranges(*c),
                    device.get_channel_config(*c),
                )
            })
            .collect::<Vec<_>>();

        channels.sort_by(|(a, _, _), (b, _, _)| a.cmp(b));

        let mut channel_options = channels
            .iter()
            .map(|(ch, ranges, config)| {
                if let Some(ranges) = ranges {
                    if let Some(config) = config {
                        format!(
                            "Channel {} - {}",
                            ch,
                            style(format!("{} / {:?}", config.range, config.coupling)).green()
                        )
                    } else if ranges.is_empty() {
                        format!(
                            "Channel {} - {}",
                            ch,
                            style("No Probe connected").red().bold()
                        )
                    } else {
                        format!("Channel {} - {}", ch, style("Disabled"))
                    }
                } else {
                    format!(
                        "Channel {} - {}",
                        ch,
                        style("Disabled due to power constraints").red().bold()
                    )
                }
            })
            .collect::<Vec<String>>();

        if channels.iter().any(|(_, _, config)| config.is_some()) {
            channel_options.push("Channel configuration complete".to_string());
        }

        let ch_selection = Select::with_theme(&better_theme())
            .with_prompt(&format!(
                "{}",
                style("Configure channels").blue().underlined().bold()
            ))
            .default(0)
            .items(&channel_options[..])
            .interact()
            .unwrap();

        if ch_selection >= channels.len() {
            return channels
                .iter()
                .map(|(ch, _, config)| config.map(|c| (*ch, c.range.get_units().short)))
                .flatten()
                .collect();
        }

        let (edit_channel, ranges, _) = channels[ch_selection].clone();

        if let Some(ranges) = ranges {
            if ranges.is_empty() {
                println!(
                    "{} cannot be configured with no probe connected",
                    edit_channel
                );

                continue;
            }

            if let Some(range) = select_range(&ranges) {
                device.enable_channel(edit_channel, range, PicoCoupling::DC);
            } else {
                device.disable_channel(edit_channel);
            }
        } else {
            println!(
                "{} cannot be configured as it's disabled due to power constraints",
                edit_channel
            );
        }
    }
}

pub fn get_colour(ch: PicoChannel) -> Style {
    match ch {
        PicoChannel::A => Style::new().blue(),
        PicoChannel::B => Style::new().red(),
        PicoChannel::C => Style::new().green(),
        PicoChannel::D => Style::new().yellow(),
        PicoChannel::E => Style::new().magenta(),
        PicoChannel::F => Style::new().white(),
        PicoChannel::G => Style::new().cyan(),
        _ => Style::new().white(),
    }
}

pub struct CaptureStats {
    rate_calc: RateCalc,
    ch_units: HashMap<PicoChannel, String>,
    state: web::Data<Mutex<AppState>>,
}

impl CaptureStats {
    pub fn new(
        ch_units: HashMap<PicoChannel, String>,
        state: web::Data<Mutex<AppState>>,
    ) -> Arc<Self> {
        Arc::new(CaptureStats {
            rate_calc: RateCalc::new(Duration::from_secs(5)),
            ch_units,
            state,
        })
    }
}

impl NewDataHandler for CaptureStats {
    #[tracing::instrument(level = "trace", skip(self, event))]
    fn handle_event(&self, event: &StreamingEvent) {
        let mut state_unlocked = self.state.lock();
        if state_unlocked.recording {
            let mut data: Vec<(PicoChannel, usize, Vec<f64>, String)> = event
                .channels
                .iter()
                .map(|(ch, v)| {
                    (
                        *ch,
                        v.samples.len(),
                        v.scale_samples(),
                        self.ch_units
                            .get(&ch)
                            .unwrap_or(&"".to_string())
                            .to_string(),
                    )
                })
                .collect();

            data.sort_by(|a, b| a.0.cmp(&b.0));

            // println!("Data Len {:?}",data);

            for channel in data.clone() {
                let key = channel.0;

                (*state_unlocked
                    .voltage_stream
                    .entry(key.clone())
                    .or_insert_with(|| Vec::new()))
                .extend(channel.2.clone().into_iter());
                (*state_unlocked
                    .voltage_queue
                    .entry(key.clone())
                    .or_insert_with(|| VecDeque::new()))
                .extend(channel.2.clone().into_iter());
            }
        }

        state_unlocked.streaming_speed = self.rate_calc.get_value(event.length);

        let state = self.state.clone();

        pin! {
            let _fut = split_data(state);
        }

        drop(state_unlocked);
        // println!("Time taking for data collection is {:?} ms",Instant::now()-start);
    }
}

async fn split_data(state: web::Data<Mutex<AppState>>) {
    let mut locked_state = state.lock();
    let pico_sped = locked_state.streaming_speed;
    let arduino_hz = ConstConfig::get_config().arduino_hz;
    let mut channels_block = HashMap::new();

    for (channel, data) in locked_state.voltage_stream.iter_mut() {
        while data.len() > arduino_hz {
            let block: Vec<f64> = data.drain(0..arduino_hz).collect();
            channels_block
                .entry(channel.to_owned())
                .or_insert_with(|| vec![])
                .push(block);
        }
    }

    drop(locked_state);

    let mut map_to_be_processed = vec![];

    let channels: Vec<PicoChannel> = channels_block.keys().map(|a| a.to_owned()).collect();

    if channels.is_empty() {
        return;
    }

    for i in 0..channels_block.get(&channels[0]).unwrap().len() {
        let mut blocks = HashMap::new();

        for (a, vals) in channels_block.iter() {
            blocks.insert(a.to_owned(), vals[i].to_owned());
        }

        map_to_be_processed.push(blocks);
    }

    for a in map_to_be_processed.iter() {
        let data = match split_into_virt_channels(&a, pico_sped) {
            Ok(data) => write_data(data, Some(format!("{}", Local::now().format("%F_%T"),))),
            Err(_) => todo!(),
        };
    }
}

pub fn get_capture_rate() -> u32 {
    let rates: Vec<u32> = vec![
        1_000,
        10_000,
        100_000,
        1_000_000,
        5_000_000,
        10_000_000,
        20_000_000,
        50_000_000,
        100_000_000,
    ];

    let rate_options: Vec<String> = rates
        .iter()
        .map(|r| {
            let sig = metric::Signifix::try_from(*r).unwrap();
            format!(
                "{:>8}",
                format!("{} {}S/s", sig.integer(), sig.symbol().unwrap_or(""))
            )
        })
        .collect();

    let rate_selection = Select::with_theme(&better_theme())
        .with_prompt(&format!(
            "{}",
            style("Select capture rate").blue().underlined().bold()
        ))
        .default(0)
        .items(&rate_options[..])
        .interact()
        .unwrap();

    rates[rate_selection]
}

pub fn select_range(ranges: &[PicoRange]) -> Option<PicoRange> {
    let mut range_options: Vec<String> = ranges.iter().map(|r| format!("{}", r)).collect();
    range_options.push("Disabled".to_string());

    let range_selection = Select::with_theme(&better_theme())
        .with_prompt(&format!(
            "{}",
            style("Select Range").blue().underlined().bold()
        ))
        .default(0)
        .items(&range_options[..])
        .interact()
        .unwrap();

    if range_selection >= ranges.len() {
        None
    } else {
        Some(ranges[range_selection])
    }
}

pub fn print_stats(state: &web::Data<Mutex<AppState>>) {
    let unlocked_state = state.lock();
    // Streaming Rate
    println!(
        "{} @ {}",
        format!("{}", style("Streaming").green().bold()),
        format!(
            "{}",
            style(format!(
                "{}S/s",
                match metric::Signifix::try_from(unlocked_state.streaming_speed) {
                    Ok(v) => format!("{}", v),
                    Err(metric::Error::OutOfLowerBound(_)) => "0".to_string(),
                    _ => panic!("unknown error"),
                }
            ))
            .bold()
        )
    );
    // Data Collected
    println!(
        "{} -> {}",
        format!("{}", style("Data Collected").green().bold()),
        style(format!("{} samples", &unlocked_state.voltage_stream.len())).bold()
    );
    // Attempt to get memory usage
    let memory_usage = format!(
        "{}",
        match statm_self() {
            Ok(s) => format!(
                "{}B",
                match metric::Signifix::try_from(s.data) {
                    Ok(v) => format!("{}", v),
                    Err(metric::Error::OutOfLowerBound(_)) => "0".to_string(),
                    _ => panic!("unknown error"),
                }
            ),
            Err(_) => format!("<Unsupported OS>"),
        }
    );
    println!(
        "{} -> {}",
        format!("{}", style("Memory Usage").green().bold()),
        style(memory_usage).bold()
    );
    drop(unlocked_state)
}
pub fn clear_and_get_memory(
    state: web::Data<Mutex<AppState>>,
    completely_clear: bool,
) -> HashMap<PicoChannel, VecDeque<f64>> {
    let mut state_unlocked = state.lock();
    let voltages = state_unlocked.voltage_queue.clone();
    if completely_clear {
        state_unlocked.voltage_queue.clear();
        state_unlocked.voltage_stream.clear();
    } else {
        for channel in voltages.keys() {
            state_unlocked
                .voltage_queue
                .get_mut(channel)
                .unwrap()
                .clear()
        }
    }

    drop(state_unlocked);
    return voltages;
}
