use crate::ConstConfig;

use std::{collections::HashMap, ops::Add};

use pico_sdk::prelude::PicoChannel;

pub type VirtChannel = usize;
pub type VirtSamples = HashMap<VirtChannel, f64>;

use console::{style, Term};
use chrono::prelude::Local;


use std::fs::File;
use std::io::Write;

#[derive(Debug)]
pub enum VirtChannelError {
    NotEnoughData,
    NoSyncPulse,
}

// fn generate_virtal_sample_layout() -> VirtSamples {
//     let virt_channels: VirtSamples = HashMap::new();
//     for i in 0..ConstConfig::get_config().virt_channel_count {
//         virt_channels.insert(i, None);
//     }
//     return virt_channels;
// }

pub fn split_into_virt_channels(
    raw_data: &HashMap<PicoChannel, Vec<f64>>,
    picoscope_sample_speed: u64,
) -> Result<Vec<VirtSamples>, VirtChannelError> {
    let const_config = ConstConfig::get_config();
    let mut final_virtual_samples: Vec<VirtSamples> = vec![];

    // Estimate samples per arudino switch
    let est_sample_width: usize = (picoscope_sample_speed as usize) / const_config.arduino_hz;

    // Find High points in data (indicateding a sync pulse)
    let mut sync_pulses: HashMap<PicoChannel, Vec<usize>> = HashMap::new();
    for (channel, data) in raw_data {

        match find_sync_pulse(&data, est_sample_width) {
            Ok(pulses) => sync_pulses.insert(*channel, pulses),
            Err(err) => return Err(err),
        };
    }
    for (_, data) in sync_pulses.clone() {
        if data.len() < 2 {
            return Err(VirtChannelError::NotEnoughData);
        }
    }

    // Determine points to extract from data, and extracts an averaged result
    let mut virt_channel_samples: HashMap<PicoChannel, Vec<VirtSamples>> = HashMap::new();
    for (channel, data) in sync_pulses {
        virt_channel_samples.insert(
            channel,
            determine_virt_channel_samples(
                &data,
                const_config.virt_channel_count,
                &raw_data[&channel],
            ),
        );
    }
    // Flatten Pico channels into Virt Channels
    for (_, data) in virt_channel_samples {
        final_virtual_samples.extend(data);
    }

    return Ok(final_virtual_samples);
}

/// Takes Picoscope data and determines the center of all the sync pulses
fn find_sync_pulse(
    input: &Vec<f64>,
    est_sample_width: usize,
) -> Result<Vec<usize>, VirtChannelError> {
    let const_config = ConstConfig::get_config();

    let mut final_sync_points: Vec<usize> = vec![];

    // Find all datapoints beyond threshold
    let mut elevated_points: Vec<usize> = vec![];
    for (index, data_point) in input.iter().enumerate() {
        if *data_point > const_config.sync_point_threshold {
            elevated_points.push(index)
        }
        if *data_point > 5f64 {
            println!("WHAT {}",*data_point);
        }
    }
    println!(
        "\nElevated Points {} vs points {}",
        elevated_points.len(),
        input.len()
    );
    if elevated_points.len() == 0 {
        return Err(VirtChannelError::NoSyncPulse);
    }

    // Classify points into blocks
    let mut raw_block_points: Vec<(usize, usize)> = vec![];
    let mut current_block: (usize, usize) = (0, 0); // (start,end)
    let upper_sample_width =
        ((est_sample_width as f32) * (1f32 + const_config.arduino_hz_tolerance)).round() as usize;
    for point in elevated_points.iter() {
        if point - current_block.0 > upper_sample_width {
            if current_block.1 < current_block.0 {
                println!("What...");
                println!("block 1 {}, block 0 {}", current_block.1, current_block.0)
            }
            raw_block_points.push(current_block);
            current_block = (point.clone(), point.clone());
        } else {
            current_block.1 = point.clone();
        }
    }
    // Scan and find midpoints of good blocks
    // Algorithm is jank so it either creates an empty block at start or an empty one, so we remove it
    raw_block_points.remove(0);
    let lower_sample_width =
        ((est_sample_width as f32) * (1f32 - const_config.arduino_hz_tolerance)).round() as usize;
    for block in raw_block_points.clone() {
        if block.1 - block.0 > lower_sample_width {
            let mid_point = block.0 + ((block.1 - block.0) / 2);
            final_sync_points.push(mid_point);
        } else {
            println!("Found bad block from noise {} ", est_sample_width)
        }
    }
    return Ok(final_sync_points);
}

/// Calculates the mid-point of all the virtual channels.
fn determine_virt_channel_samples(
    sync_points: &Vec<usize>,
    virt_channel_count: usize,
    full_data: &Vec<f64>,
) -> Vec<VirtSamples> {
    let mut virt_channel_samples: Vec<VirtSamples> = vec![];
    // let mut cumulative_diff: usize = 0;

    // Find spacing for data points in between sync points
    // round = Index of the current element
    // pulse_index = Index of the sync point in the full data set
    for (round, pulse_index) in (&sync_points[0..(sync_points.len() - 2)])
        .iter()
        .enumerate()
    {
        let samples: Vec<f64> = full_data[*pulse_index..sync_points[round + 1]].to_vec();
        dump_data(samples);
        // The spacing between the two sync points
        let diff = sync_points[round + 1] - pulse_index;
        // There is the virt channel count + 1 in one diff
        let spacing = diff / (virt_channel_count+1);
        virt_channel_samples.push(HashMap::new());
        // loop through virt channels
        for i in 0..virt_channel_count {
            let virt_channel_index = pulse_index + spacing * (i + 1);
            virt_channel_samples.last_mut().unwrap().insert(
                i,
                get_average_sample(&virt_channel_index, &full_data, &spacing),
            );
        }
    }

    // Find averages to
    // let average_diff = cumulative_diff / (sync_points.len() - 2);
    // let average_spacing = average_diff / virt_channel_count;

    return virt_channel_samples;
}

/// Takes a fancy average of the sampled data
fn get_average_sample(index: &usize, full_dataset: &Vec<f64>, width_of_channel: &usize) -> f64 {
    let const_config = ConstConfig::get_config();

    let width_of_average: usize = width_of_channel / 3;
    let mut samples: Vec<f64> =
        full_dataset[(index - width_of_average)..(index + width_of_average + 1)].to_vec();
    // Can't use default .sort() because rust small brain with floats
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // Haven't done statistics properly yet, so here goes...
    // If the range is too high (EG: Error), we use median, otherwise a mean would be more accurate
    if samples.last().unwrap() - samples.first().unwrap()
        < const_config.virt_channel_noise_threshold
    {
        // Sensitive to skews, insensitive to noise
        samples.iter().fold(0.0, Add::add) as f64 / samples.len() as f64
    } else {
        // Sensitive to noise, insensitive to skews
        samples[samples.len() / 2]

    }
}

fn dump_data(data: Vec<f64>) {
    let cwd = std::env::current_dir().unwrap();

    let save_path = cwd.join(format!("data_output/_Test_{}.csv", Local::now().format("%+")));
    let mut file: File = File::create(save_path).unwrap();
    let mut writer = csv::Writer::from_writer(vec![]);
    for value in &data {
        
        writer.write_record( std::iter::once(value.clone().to_string())).unwrap();
       

    }
    let csv_data = String::from_utf8(writer.into_inner().unwrap()).unwrap();
    file.write_all(csv_data.as_bytes()).unwrap();
    let mut samples = data.clone();
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    // println!("{:?}",samples.iter().fold(0.0, Add::add) as f64 / samples.len() as f64);
}