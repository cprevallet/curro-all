use chrono::{DateTime, TimeZone, Utc};
use dashmap::DashMap;
use rayon::prelude::*;
use std::fs::File;
use std::io::Read;
use std::io::{BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::WalkDir;

pub struct SessionStats {
    pub distance: f64,
    pub calories: u16,
    pub duration: f64,
    pub enhanced_speed: f64,
    pub ascent: u16,
    pub descent: u16,
}

impl Default for SessionStats {
    fn default() -> Self {
        Self {
            distance: 0.0,
            calories: 0,
            duration: 0.0,
            enhanced_speed: 0.0,
            ascent: 0,
            descent: 0,
        }
    }
}

pub fn extract_session_data(
    path: &Path,
) -> Result<SessionStats, Box<dyn std::error::Error + Send + Sync>> {
    let mut file = File::open(path)?;
    let messages = fitparser::from_reader(&mut file)?;
    let mut stats = SessionStats::default();

    for message in messages {
        if message.kind() == fitparser::profile::field_types::MesgNum::Session {
            for field in message.fields() {
                match field.name() {
                    "total_distance" => {
                        stats.distance = match field.value() {
                            fitparser::Value::Float32(v) => *v as f64,
                            fitparser::Value::Float64(v) => *v,
                            _ => 0.0,
                        };
                    }
                    "total_calories" => {
                        if let fitparser::Value::UInt16(v) = field.value() {
                            stats.calories = *v;
                        }
                    }
                    "total_elapsed_time" => {
                        stats.duration = match field.value() {
                            fitparser::Value::Float32(v) => *v as f64,
                            fitparser::Value::Float64(v) => *v,
                            _ => 0.0,
                        };
                    }
                    "enhanced_avg_speed" => {
                        stats.enhanced_speed = match field.value() {
                            fitparser::Value::Float32(v) => *v as f64,
                            fitparser::Value::Float64(v) => *v,
                            _ => 0.0,
                        };
                    }
                    "total_ascent" => {
                        if let fitparser::Value::UInt16(v) = field.value() {
                            stats.ascent = *v;
                        }
                    }
                    "total_descent" => {
                        if let fitparser::Value::UInt16(v) = field.value() {
                            stats.descent = *v;
                        }
                    }
                    _ => {}
                }
            }
            return Ok(stats);
        }
    }
    Ok(stats)
}

pub fn get_files_in_range(
    map: &Arc<DashMap<DateTime<Utc>, PathBuf>>,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Vec<(DateTime<Utc>, PathBuf)> {
    map.iter()
        .filter(|entry| {
            let ts = entry.key();
            *ts >= start && *ts <= end
        })
        .map(|entry| (*entry.key(), entry.value().clone()))
        .collect()
}

pub fn process_fit_directory(dir: &str) -> Arc<DashMap<DateTime<Utc>, PathBuf>> {
    let map = Arc::new(DashMap::new());
    let paths: Vec<PathBuf> = WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("fit"))
                .unwrap_or(false)
        })
        .map(|e| e.into_path())
        .collect();

    paths.into_par_iter().for_each(|path| {
        if let Ok(ts) = extract_timestamp_fast(&path) {
            //   This is WAY faster but we seem to be missing some test cases.
            //  TODO Need to investigate.
            // if let Ok(ts) = extract_timestamp_bit_level(&path) {
            map.insert(ts, path);
        }
    });
    map
}

fn extract_timestamp_fast(
    path: &Path,
) -> Result<DateTime<Utc>, Box<dyn std::error::Error + Send + Sync>> {
    let file = File::open(path)?;
    let mut reader = file.take(2048);
    match fitparser::from_reader(&mut reader) {
        Ok(messages) => find_ts_in_vec(&messages),
        Err(_) => {
            let mut full_file = File::open(path)?;
            let messages = fitparser::from_reader(&mut full_file)?;
            find_ts_in_vec(&messages)
        }
    }
}

fn find_ts_in_vec(
    messages: &[fitparser::FitDataRecord],
) -> Result<DateTime<Utc>, Box<dyn std::error::Error + Send + Sync>> {
    for message in messages {
        if let Some(field) = message.fields().iter().find(|f| f.name() == "time_created") {
            if let fitparser::Value::Timestamp(ts) = field.value() {
                return Ok((*ts).into());
            }
        }
    }
    Err("Timestamp not found".into())
}

// This is buggy - seems to be missing some valid files picked up with the other approach.
// Extracts the timestamp by manually parsing the binary File ID message.
// This avoids the overhead of a full FIT parser.
// pub fn extract_timestamp_bit_level(
//     path: &Path,
// ) -> Result<DateTime<Utc>, Box<dyn std::error::Error + Send + Sync>> {
//     let file = File::open(path)?;
//     let mut reader = BufReader::new(file);

//     // 1. Read Header Size (First byte)
//     let mut header_size_buf = [0u8; 1];
//     reader.read_exact(&mut header_size_buf)?;
//     let header_size = header_size_buf[0];

//     // 2. Skip to the start of Data Records
//     reader.seek(SeekFrom::Start(header_size as u64))?;

//     // 3. Parse Record Header
//     // Most Garmin File ID messages use a "Normal Header" (bit 7 is 0)
//     let mut record_header = [0u8; 1];
//     reader.read_exact(&mut record_header)?;

//     // 4. Look for the File ID message
//     // In the FIT protocol, the first message is almost always the File ID (Global ID 0).
//     // It contains the 'time_created' field at a specific offset.
//     // We expect a Definition Message (bit 6 set) or a Data Message.

//     // For extreme speed, we can search for the first Timestamp (u32)
//     // that looks like a valid Garmin date (seconds since 1989-12-31).
//     let mut buffer = [0u8; 128];
//     reader.read_exact(&mut buffer)?;

//     for i in 0..(buffer.len() - 4) {
//         let val = u32::from_le_bytes([buffer[i], buffer[i + 1], buffer[i + 2], buffer[i + 3]]);

//         // Garmin timestamps are seconds since UTC 00:00:00 Dec 31, 1989.
//         // A timestamp of ~1.1 billion represents late 2024/2025.
//         if val > 1_000_000_000 && val < 1_500_000_000 {
//             let unix_ts = val as i64 + 631_065_600; // Offset to Unix Epoch
//             return Ok(Utc.timestamp_opt(unix_ts, 0).unwrap());
//         }
//     }

//     Err("Could not find valid timestamp in bitstream".into())
// }
