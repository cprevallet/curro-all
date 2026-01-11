use chrono::{DateTime, Utc};
use dashmap::DashMap;
use rayon::prelude::*;
use std::fs::File;
use std::io::Read;
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
