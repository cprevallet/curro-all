use chrono::{DateTime, Utc};
use plotters::prelude::*;
use rayon::prelude::*;
use std::path::PathBuf;

// Import types from our data module
use crate::data::{SessionStats, extract_session_data};

/// Extract the data, sort and display in the terminal.
pub fn print_activity_summaries(results: &[(DateTime<Utc>, PathBuf)]) {
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

/// Generates a LineSeries chart for a specific metric.
pub fn plot_session_metric(
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

    // 5. Draw Scatter Points
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
