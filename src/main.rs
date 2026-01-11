use chrono::{DateTime, TimeZone, Utc};
use dashmap::DashMap;
use plotters::prelude::*;
use rayon::prelude::*;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::WalkDir;

fn main() {
    let target_dir = "/home/craig/Documents/garmin/";
    let lookup = process_fit_directory(target_dir);

    let start_date = Utc.with_ymd_and_hms(2025, 7, 1, 0, 0, 0).unwrap();
    let end_date = Utc.with_ymd_and_hms(2025, 12, 31, 23, 59, 59).unwrap();

    let result = get_files_in_range(&lookup, start_date, end_date);

    // Print the list of activities and their distances
    print_activity_summaries(&result);

    // Generate Distance Graph (Miles)
    plot_session_metric(&result, "Distance", "distance_chart.png", "Miles", |s| {
        s.distance / 1000.0 * 0.621371
    })
    .unwrap();

    // Generate Calories Graph
    plot_session_metric(&result, "Calories", "calories_chart.png", "kcal", |s| {
        s.calories as f64
    })
    .unwrap();

    // Generate Ascent Graph (Feet)
    plot_session_metric(&result, "Elevation Gain", "ascent_chart.png", "Feet", |s| {
        s.ascent as f64 * 3.28084
    })
    .unwrap();

    // Generate Duration Graph (Minutes)
    plot_session_metric(&result, "Duration", "duration_chart.png", "Minutes", |s| {
        s.duration / 60.0
    })
    .unwrap();

    // Generate Average Speed Graph (MPH)
    plot_session_metric(&result, "Average Speed", "speed_chart.png", "MPH", |s| {
        s.enhanced_speed * 2.23694 // m/s to mph
    })
    .unwrap();

    // Generate Descent Graph (Feet)
    plot_session_metric(
        &result,
        "Elevation Loss",
        "descent_chart.png",
        "Feet",
        |s| s.descent as f64 * 3.28084,
    )
    .unwrap();
}

fn plot_session_metric(
    results: &[(DateTime<Utc>, PathBuf)],
    metric_name: &str,
    file_name: &str,
    unit_label: &str,
    value_extractor: fn(&SessionStats) -> f64,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Prepare and sort data
    let mut data: Vec<(DateTime<Utc>, f64)> = results
        .into_par_iter()
        .map(|(ts, path)| {
            let stats = extract_session_data(path).unwrap_or_default();
            (*ts, value_extractor(&stats))
        })
        .collect();

    data.sort_by_key(|(ts, _)| *ts);

    if data.is_empty() {
        return Ok(());
    }

    // 2. Set up the drawing area
    let root = BitMapBackend::new(file_name, (1024, 768)).into_drawing_area();
    root.fill(&WHITE)?;

    let (start_date, end_date) = (data.first().unwrap().0, data.last().unwrap().0);
    let max_val = data.iter().map(|(_, v)| *v).fold(0.0, f64::max) * 1.1;

    // 3. Build the chart
    let mut chart = ChartBuilder::on(&root)
        .caption(format!("{} over Time", metric_name), ("sans-serif", 40))
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(start_date..end_date, 0.0..max_val)?;

    chart
        .configure_mesh()
        .x_labels(10)
        // This line formats the Utc DateTime to just YYYY-MM-DD
        .x_label_formatter(&|d| d.format("%Y-%m-%d").to_string())
        .y_desc(unit_label)
        .draw()?;
    // 4. Draw the Line Series
    chart
        .draw_series(LineSeries::new(
            data.iter().map(|(date, val)| (*date, *val)),
            &RED,
        ))?
        .label(metric_name)
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

    // 5. Draw Scatter Points (Optional: makes individual activities visible)
    chart.draw_series(
        data.iter()
            .map(|(date, val)| Circle::new((*date, *val), 3, RED.filled())),
    )?;

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;

    root.present()?;
    println!("Chart saved to {}", file_name);
    Ok(())
}
struct SessionStats {
    distance: f64,
    calories: u16,
    duration: f64,
    enhanced_speed: f64, // Max or avg speed (m/s)
    ascent: u16,         // Total ascent (meters)
    descent: u16,        // Total descent (meters)
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

// Retrieve a set of fields from a set of files.
fn extract_session_data(
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
                        // Usually more accurate than 'avg_speed'
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

// Extract the data, sort and display.
fn print_activity_summaries(results: &[(DateTime<Utc>, PathBuf)]) {
    println!(
        "{:<25} | {:<8} | {:<5} | {:<7} | {:<7} | {:<7} | {:<7}",
        "Date & Time", "Dist(mi)", "Cal", "Time", "mph", "Asc(ft)", "Des(ft)"
    );
    println!("{:-<95}", "");

    let mut summaries: Vec<(DateTime<Utc>, SessionStats)> = results
        .into_par_iter()
        .map(|(ts, path)| (*ts, extract_session_data(path).unwrap_or_default()))
        .collect();

    summaries.sort_by_key(|(ts, _)| *ts);

    for (ts, stats) in summaries {
        // Conversions
        let miles = stats.distance / 1000.0 * 0.621371;
        let mph = stats.enhanced_speed * 2.23694; // m/s to mph
        let ascent_ft = stats.ascent as f64 * 3.28084;
        let descent_ft = stats.descent as f64 * 3.28084;
        let mins = stats.duration / 60.0;

        println!(
            "{:<25} | {:>8.2} | {:>5} | {:>6.1}m | {:>7.1} | {:>7.0} | {:>7.0}",
            ts.format("%Y-%m-%d").to_string(),
            miles,
            stats.calories,
            mins,
            mph,
            ascent_ft,
            descent_ft
        );
    }
}
// Filters the DashMap for files within the inclusive range [start, end]
// In other words, find a filename between a given set of datetimes.
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

// Create a hash table of filename keyed off of file creation time.
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

// Retrieve a file's creation timestamp quickly.
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

// Helper function for extract_timestamp_fast.
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
