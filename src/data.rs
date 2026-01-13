use crate::i18n::tr;
use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
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
    // map: &Arc<DashMap<DateTime<Utc>, PathBuf>>,
    map: &DashMap<DateTime<Utc>, PathBuf>,
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

pub fn process_fit_directory(pathbuf: &PathBuf) -> Arc<DashMap<DateTime<Utc>, PathBuf>> {
    let dir = pathbuf.to_str().unwrap();
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeBucket {
    OneWeek,
    TwoWeeks,
    ThreeWeeks,
    FourWeeks,

    // This Year
    JanuaryThisYear,
    FebruaryThisYear,
    MarchThisYear,
    AprilThisYear,
    MayThisYear,
    JuneThisYear,
    JulyThisYear,
    AugustThisYear,
    SeptemberThisYear,
    OctoberThisYear,
    NovemberThisYear,
    DecemberThisYear,

    // Last Year
    JanuaryLastYear,
    FebruaryLastYear,
    MarchLastYear,
    AprilLastYear,
    MayLastYear,
    JuneLastYear,
    JulyLastYear,
    AugustLastYear,
    SeptemberLastYear,
    OctoberLastYear,
    NovemberLastYear,
    DecemberLastYear,
}

pub fn get_time_range(bucket: TimeBucket) -> (DateTime<Utc>, DateTime<Utc>) {
    let now = Utc::now();
    let current_year = now.year();

    match bucket {
        // --- WEEKLY LOGIC (A..D) ---
        TimeBucket::OneWeek
        | TimeBucket::TwoWeeks
        | TimeBucket::ThreeWeeks
        | TimeBucket::FourWeeks => {
            let sunday_count = match bucket {
                TimeBucket::OneWeek => 1,
                TimeBucket::TwoWeeks => 2,
                TimeBucket::ThreeWeeks => 3,
                TimeBucket::FourWeeks => 4,
                _ => unreachable!(),
            };

            let today_start = Utc
                .with_ymd_and_hms(current_year, now.month(), now.day(), 0, 0, 0)
                .unwrap();
            let days_since_sunday = now.weekday().num_days_from_sunday();
            let most_recent_sunday = today_start - Duration::days(days_since_sunday as i64);

            // Start from the specific Sunday at 00:00:00
            let start_ts = most_recent_sunday - Duration::weeks(sunday_count - 1);

            (start_ts, now)
        }

        // --- MONTHLY LOGIC (Current and Previous Year) ---
        _ => {
            let (month_num, target_year) = match bucket {
                // This Year
                TimeBucket::JanuaryThisYear => (1, current_year),
                TimeBucket::FebruaryThisYear => (2, current_year),
                TimeBucket::MarchThisYear => (3, current_year),
                TimeBucket::AprilThisYear => (4, current_year),
                TimeBucket::MayThisYear => (5, current_year),
                TimeBucket::JuneThisYear => (6, current_year),
                TimeBucket::JulyThisYear => (7, current_year),
                TimeBucket::AugustThisYear => (8, current_year),
                TimeBucket::SeptemberThisYear => (9, current_year),
                TimeBucket::OctoberThisYear => (10, current_year),
                TimeBucket::NovemberThisYear => (11, current_year),
                TimeBucket::DecemberThisYear => (12, current_year),

                // Last Year
                TimeBucket::JanuaryLastYear => (1, current_year - 1),
                TimeBucket::FebruaryLastYear => (2, current_year - 1),
                TimeBucket::MarchLastYear => (3, current_year - 1),
                TimeBucket::AprilLastYear => (4, current_year - 1),
                TimeBucket::MayLastYear => (5, current_year - 1),
                TimeBucket::JuneLastYear => (6, current_year - 1),
                TimeBucket::JulyLastYear => (7, current_year - 1),
                TimeBucket::AugustLastYear => (8, current_year - 1),
                TimeBucket::SeptemberLastYear => (9, current_year - 1),
                TimeBucket::OctoberLastYear => (10, current_year - 1),
                TimeBucket::NovemberLastYear => (11, current_year - 1),
                TimeBucket::DecemberLastYear => (12, current_year - 1),
                _ => unreachable!(),
            };

            let start_ts = Utc
                .with_ymd_and_hms(target_year, month_num, 1, 0, 0, 0)
                .unwrap();

            // Calculate end of month (last second of the month)
            let end_ts = if month_num == 12 {
                Utc.with_ymd_and_hms(target_year + 1, 1, 1, 0, 0, 0)
                    .unwrap()
                    - Duration::seconds(1)
            } else {
                Utc.with_ymd_and_hms(target_year, month_num + 1, 1, 0, 0, 0)
                    .unwrap()
                    - Duration::seconds(1)
            };

            (start_ts, end_ts)
        }
    }
}
impl TimeBucket {
    // This provides the labels for the DropDown
    pub fn all_variants() -> &'static [TimeBucket] {
        use TimeBucket::*;
        &[
            OneWeek,
            TwoWeeks,
            ThreeWeeks,
            FourWeeks,
            JanuaryThisYear,
            FebruaryThisYear,
            MarchThisYear,
            AprilThisYear,
            MayThisYear,
            JuneThisYear,
            JulyThisYear,
            AugustThisYear,
            SeptemberThisYear,
            OctoberThisYear,
            NovemberThisYear,
            DecemberThisYear,
            JanuaryLastYear,
            FebruaryLastYear,
            MarchLastYear,
            AprilLastYear,
            MayLastYear,
            JuneLastYear,
            JulyLastYear,
            AugustLastYear,
            SeptemberLastYear,
            OctoberLastYear,
            NovemberLastYear,
            DecemberLastYear,
        ]
    }

    pub fn get_label(&self) -> String {
        let now = Utc::now();
        let this_year = now.year();
        let last_year = this_year - 1;

        match self {
            // Weekly Variants
            TimeBucket::OneWeek => tr("ONE_WEEK", None),
            TimeBucket::TwoWeeks => tr("TWO_WEEKS", None),
            TimeBucket::ThreeWeeks => tr("THREE_WEEKS", None),
            TimeBucket::FourWeeks => tr("FOUR_WEEKS", None),

            // This Year Variants
            TimeBucket::JanuaryThisYear => format!("{} {}", tr("JANUARY", None), this_year),
            TimeBucket::FebruaryThisYear => format!("{} {}", tr("FEBRUARY", None), this_year),
            TimeBucket::MarchThisYear => format!("{} {}", tr("MARCH", None), this_year),
            TimeBucket::AprilThisYear => format!("{} {}", tr("APRIL", None), this_year),
            TimeBucket::MayThisYear => format!("{} {}", tr("MAY", None), this_year),
            TimeBucket::JuneThisYear => format!("{} {}", tr("JUNE", None), this_year),
            TimeBucket::JulyThisYear => format!("{} {}", tr("JULY", None), this_year),
            TimeBucket::AugustThisYear => format!("{} {}", tr("AUGUST", None), this_year),
            TimeBucket::SeptemberThisYear => format!("{} {}", tr("SEPTEMBER", None), this_year),
            TimeBucket::OctoberThisYear => format!("{} {}", tr("OCTOBER", None), this_year),
            TimeBucket::NovemberThisYear => format!("{} {}", tr("NOVEMBER", None), this_year),
            TimeBucket::DecemberThisYear => format!("{} {}", tr("DECEMBER", None), this_year),

            // Last Year Variants
            TimeBucket::JanuaryLastYear => format!("{} {}", tr("JANUARY", None), last_year),
            TimeBucket::FebruaryLastYear => format!("{} {}", tr("FEBRUARY", None), last_year),
            TimeBucket::MarchLastYear => format!("{} {}", tr("MARCH", None), last_year),
            TimeBucket::AprilLastYear => format!("{} {}", tr("APRIL", None), last_year),
            TimeBucket::MayLastYear => format!("{} {}", tr("MAY", None), last_year),
            TimeBucket::JuneLastYear => format!("{} {}", tr("JUNE", None), last_year),
            TimeBucket::JulyLastYear => format!("{} {}", tr("JULY", None), last_year),
            TimeBucket::AugustLastYear => format!("{} {}", tr("AUGUST", None), last_year),
            TimeBucket::SeptemberLastYear => format!("{} {}", tr("SEPTEMBER", None), last_year),
            TimeBucket::OctoberLastYear => format!("{} {}", tr("OCTOBER", None), last_year),
            TimeBucket::NovemberLastYear => format!("{} {}", tr("NOVEMBER", None), last_year),
            TimeBucket::DecemberLastYear => format!("{} {}", tr("DECEMBER", None), last_year),
        }
    }
}

pub fn get_filtered_variants() -> Vec<TimeBucket> {
    // 1. Get current date info
    let now = Utc::now();
    let current_month = now.month();
    let filtered_variants: Vec<TimeBucket> = TimeBucket::all_variants()
        .iter()
        .filter(|bucket| {
            match bucket {
                // Always keep weekly ranges and Last Year months
                TimeBucket::OneWeek
                | TimeBucket::TwoWeeks
                | TimeBucket::ThreeWeeks
                | TimeBucket::FourWeeks => true,

                // Check specific variants for "Last Year" (always keep)
                b if format!("{:?}", b).contains("LastYear") => true,

                // Filter "This Year" months
                TimeBucket::JanuaryThisYear => 1 <= current_month,
                TimeBucket::FebruaryThisYear => 2 <= current_month,
                TimeBucket::MarchThisYear => 3 <= current_month,
                TimeBucket::AprilThisYear => 4 <= current_month,
                TimeBucket::MayThisYear => 5 <= current_month,
                TimeBucket::JuneThisYear => 6 <= current_month,
                TimeBucket::JulyThisYear => 7 <= current_month,
                TimeBucket::AugustThisYear => 8 <= current_month,
                TimeBucket::SeptemberThisYear => 9 <= current_month,
                TimeBucket::OctoberThisYear => 10 <= current_month,
                TimeBucket::NovemberThisYear => 11 <= current_month,
                TimeBucket::DecemberThisYear => 12 <= current_month,

                _ => true, // Default fallback
            }
        })
        .cloned()
        .collect();
    filtered_variants
}
