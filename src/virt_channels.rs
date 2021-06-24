use crate::ConstConfig;

use std::{collections::HashMap, ops::Add};

use pico_sdk::prelude::PicoChannel;

pub type VirtChannel = usize;
pub type VirtSamples = HashMap<VirtChannel, f64>;

pub enum VirtChannelError {
    NotEnoughData,
}

// fn generate_virtal_sample_layout() -> VirtSamples {
//     let virt_channels: VirtSamples = HashMap::new();
//     for i in 0..ConstConfig::get_config().virt_channel_count {
//         virt_channels.insert(i, None);
//     }
//     return virt_channels;
// }

pub fn split_into_virt_channels(
    raw_data: &HashMap<PicoChannel, (usize, Vec<f64>, String)>,
    picoscope_sample_speed: u64,
) -> Result<Vec<VirtSamples>, VirtChannelError> {
    let const_config = ConstConfig::get_config();
    let final_virtual_samples: Vec<VirtSamples> = vec![];
    let mut i = 0;

    // Estimate samples per arudino switch
    let est_sample_width: usize = (picoscope_sample_speed as usize) / const_config.arudino_hz;

    // Find High points in data (indicateding a sync pulse)
    let sync_pulses: HashMap<PicoChannel, Vec<usize>> = HashMap::new();
    for (channel, data) in raw_data {
        let sync_pulses = find_sync_pulse(&data.1, est_sample_width);
    }
    if sync_pulses.len() < 2 {
        return Err(VirtChannelError::NotEnoughData);
    }

    // Determine points to extract from data, and extracts an averaged result
    let virt_channel_samples: HashMap<PicoChannel, Vec<VirtSamples>> = HashMap::new();
    for (channel, data) in sync_pulses {
        virt_channel_samples.insert(
            channel,
            determine_virt_channel_samples(
                &sync_pulses[&channel],
                const_config.virt_channel_count,
                &raw_data[&channel].1,
            ),
        );
    }
    // Flatten Pico channels into Virt Channels
    for (channel, data) in virt_channel_samples {
        final_virtual_samples.extend(data);
    }

    return Ok(final_virtual_samples);
}

/// Takes Picoscope data and determines the center of all the sync pulses
fn find_sync_pulse(input: &Vec<f64>, est_sample_width: usize) -> Vec<usize> {
    let const_config = ConstConfig::get_config();

    let final_sync_points: Vec<usize> = vec![];

    // Find all datapoints beyond threshold
    let mut elevated_points: Vec<usize> = vec![];
    for (index, data_point) in input.iter().enumerate() {
        if *data_point > const_config.sync_point_threashold {
            elevated_points.push(index)
        }
    }

    // Classifiy points into blocks
    let mut raw_block_points: Vec<(usize, usize)> = vec![];
    let mut current_block: (usize, usize) = (0, 0); // (start,end)
    let upper_sample_width =
        ((est_sample_width as f32) * (1f32 + const_config.arduino_hz_tolerence)).round() as usize;
    for (index, point) in elevated_points.iter().enumerate() {
        if index - current_block.0 > est_sample_width {
            raw_block_points.push(current_block);
            current_block.0 = index
        } else {
            current_block.1 = index
        }
    }
    raw_block_points.push(current_block);

    // Scan and find midpoints of good blocks
    // Algerithm is jank so it either creates an empty block at start or an empty one, so we remove it
    raw_block_points.remove(0);
    let lower_sample_width =
        ((est_sample_width as f32) * (1f32 - const_config.arduino_hz_tolerence)).round() as usize;
    for block in raw_block_points.clone() {
        if block.0 - block.1 > lower_sample_width {
            let mid_point = (block.0 - block.1) / 2;
            final_sync_points.push(mid_point)
        }
    }

    return final_sync_points;
}

/// Calculates the mid-point of all the virtual channels.
fn determine_virt_channel_samples(
    sync_points: &Vec<usize>,
    virt_channel_count: usize,
    full_data: &Vec<f64>,
) -> Vec<(VirtSamples)> {
    let mut virt_channel_samples: Vec<VirtSamples> = vec![];
    let mut cumulative_diff: usize = 0;

    // Find spacing for data points in between sync points
    for (round, pulse_index) in (&sync_points[1..(sync_points.len() - 1)])
        .iter()
        .enumerate()
    {
        let diff = sync_points[round - 1] - pulse_index;
        cumulative_diff += diff;
        let spacing = diff / virt_channel_count;
        virt_channel_samples.push(HashMap::new());
        // loop through virt channels
        for i in 0..virt_channel_count {
            let virt_channel_index = pulse_index + spacing * (i + 1);
            virt_channel_samples.last_mut().unwrap().insert(i, get_average_sample(&virt_channel_index,&full_data,&spacing));
        }
    }

    // Find averages to
    let average_diff = cumulative_diff / (sync_points.len() - 2);
    let average_spacing = average_diff / virt_channel_count;

    return virt_channel_samples;
}

/// Takes a fancy average of the sampled data
fn get_average_sample(index: &usize,full_dataset: &Vec<f64>,width_of_channel: &usize) -> f64 {
    let width_of_average: usize = width_of_channel/3;
    let mut samples: Vec<f64> = full_dataset[index-width_of_average..(index+width_of_average+1)].to_vec();
    // Can't use default .sort() because rust small brain with floats
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // Haven't done statistics properly yet, so here goes...
    let median = samples[samples.len()/2]; // Sensitive to noise, insensitive to skews
    let mean: f64 = samples.iter().fold(0.0, Add::add) as f64 / samples.len() as f64; // Sensitive to skews, insensitive to noise

    // So we combine them, if the mean is too far off to not be skewed, then we use the median
    let percent_dif_averages = (median-mean)/median;
    if -0.1 < percent_dif_averages && percent_dif_averages < 0.1 {
        return mean
    } else {
        return median
    }
}