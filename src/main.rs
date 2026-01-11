mod data;
mod gui;

use chrono::{TimeZone, Utc};
use data::{get_files_in_range, process_fit_directory};
use gui::{plot_session_metric, print_activity_summaries};

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
