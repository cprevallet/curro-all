use chrono::{DateTime, TimeZone, Utc};
use dashmap::DashMap;
use rayon::prelude::*;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::WalkDir;

fn main() {
    let target_dir = "/home/craig/Documents/garmin/";
    let lookup = process_fit_directory(target_dir);

    let start_date = Utc.with_ymd_and_hms(2025, 12, 1, 0, 0, 0).unwrap();
    let end_date = Utc.with_ymd_and_hms(2025, 12, 31, 23, 59, 59).unwrap();

    let result = get_files_in_range(&lookup, start_date, end_date);

    // Print the list of activities and their distances
    print_activity_summaries(&result);

    // You can still calculate the grand total afterward
    let total_meters = calculate_total_distance(&result);
    println!("\nGrand Total: {:.2} mi", total_meters / 1000.0 * 0.62);
}

/// Iterates through the filtered results and sums the total distance in meters
fn calculate_total_distance(results: &[(DateTime<Utc>, PathBuf)]) -> f64 {
    results
        .into_par_iter() // Process files in parallel for maximum speed
        .map(|(_ts, path)| extract_session_distance(path).unwrap_or(0.0))
        .sum()
}
/// Iterates through the results and prints the DateTime and Distance for each
fn print_activity_summaries(results: &[(DateTime<Utc>, PathBuf)]) {
    println!("{:<25} | {:<15}", "Date & Time", "Distance (km)");
    println!("{:-<45}", "");

    // We can use the same parallel extraction logic to gather distances quickly
    let summaries: Vec<(DateTime<Utc>, f64)> = results
        .into_par_iter()
        .map(|(ts, path)| {
            let dist = extract_session_distance(path).unwrap_or(0.0);
            (*ts, dist / 1000.0 * 0.62) // Convert meters to mi
        })
        .collect();

    // Sort by date before printing for a better user experience
    let mut sorted_summaries = summaries;
    sorted_summaries.sort_by_key(|(ts, _)| *ts);

    for (ts, dist) in sorted_summaries {
        println!("{:<25} | {:.2} mi", ts.to_rfc2822(), dist);
    }
}

fn extract_session_distance(path: &Path) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
    let mut file = File::open(path)?;
    let messages = fitparser::from_reader(&mut file)?;

    for message in messages {
        // Look for the session message (MesgNum 18)
        if message.kind() == fitparser::profile::field_types::MesgNum::Session {
            if let Some(field) = message
                .fields()
                .iter()
                .find(|f| f.name() == "total_distance")
            {
                match field.value() {
                    fitparser::Value::Float32(v) => return Ok(*v as f64),
                    fitparser::Value::Float64(v) => return Ok(*v),
                    fitparser::Value::UInt32(v) => return Ok(*v as f64),
                    _ => continue,
                }
            }
        }
    }
    Ok(0.0)
}

/// Filters the DashMap for files within the inclusive range [start, end]
fn get_files_in_range(
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

fn process_fit_directory(dir: &str) -> Arc<DashMap<DateTime<Utc>, PathBuf>> {
    // DashMap is a concurrent Split-Ordered Hash Table.
    // It is much faster than Mutex<HashMap> for high-concurrency writes.
    let map = Arc::new(DashMap::new());

    // 1. Efficiently gather all potential FIT files
    let paths: Vec<PathBuf> = WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                // Case-insensitive check for .fit extension
                .map(|ext| ext.eq_ignore_ascii_case("fit"))
                .unwrap_or(false)
        })
        .map(|e| e.into_path())
        .collect();

    // 2. Process files in parallel
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

    // Attempt the fast-path (first 2KB)
    let mut reader = file.take(2048);

    match fitparser::from_reader(&mut reader) {
        Ok(messages) => find_ts_in_vec(&messages),
        Err(_) => {
            // Fallback: If 2KB wasn't enough or header was malformed, read the whole file.
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
        // file_id message contains time_created
        if let Some(field) = message.fields().iter().find(|f| f.name() == "time_created") {
            if let fitparser::Value::Timestamp(ts) = field.value() {
                return Ok((*ts).into());
            }
        }
    }
    Err("Timestamp not found".into())
}
