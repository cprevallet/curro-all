// User interface logic - setup, drawing, formatting.

use crate::config::{ICON_NAME, PROGRAM_NAME, SETTINGSFILE, Units, load_config};
use crate::data::{PlottableData, TimeBucket, convert_session_data, get_time_range};
use crate::i18n::tr;
use dashmap::DashMap;
use directories::BaseDirs;
use gtk4::cairo::Context;
use gtk4::ffi::GTK_STYLE_PROVIDER_PRIORITY_APPLICATION;
use gtk4::glib::clone;
use gtk4::prelude::*;
use gtk4::{
    Button, DrawingArea, DropDown, Frame, HeaderBar, Image, Label, MenuButton, Orientation,
    Popover, ScrolledWindow, Spinner, StringList, StringObject, TextBuffer, TextView, gdk,
};
use libadwaita::prelude::*;
use libadwaita::{Application, ApplicationWindow, StyleManager, WindowTitle};
use plotters::prelude::*;
use plotters::style::full_palette::{BROWN, CYAN, GREY_200, GREY_400, GREY_600, GREY_800};
use plotters_cairo::CairoBackend;
use std::path::Path;
use std::rc::Rc;

use chrono::{DateTime, Utc};
use rayon::prelude::*;
use std::path::PathBuf;

// Import types from our data module
use crate::data::{SessionStats, extract_session_data, get_filtered_variants};

// #####################################################################
// ##################### OVERALL UI FUNCTIONS ##########################
// #####################################################################
//
// Widgets used for the graphical user interface.
pub struct UserInterface {
    pub settings_file: String,
    pub win: ApplicationWindow,
    pub header_bar: HeaderBar,
    pub menu_button: gtk4::MenuButton,
    pub popover: gtk4::Popover,
    pub spinner: Spinner,
    pub time_widget: DropDown,
    pub status_label: Label,
    pub menu_box: gtk4::Box,
    pub outer_box: gtk4::Box,
    pub button_box: gtk4::Box,
    pub main_pane: gtk4::Paned,
    pub btn: Button,
    pub text_view: TextView,
    pub text_buffer: TextBuffer,
    pub frame_left: Frame,
    pub frame_right: Frame,
    pub left_frame_pane: gtk4::Paned,
    pub right_frame_pane: gtk4::Paned,
    pub scrolled_window: ScrolledWindow,
    pub da_window: ScrolledWindow,
    pub curr_time_label: Label,
    pub controls_box: gtk4::Box,
    pub uom: StringList,
    pub units_widget: DropDown,
    pub about_label: String,
    pub about_btn: Button,
    pub da: DrawingArea,
    pub lookup: DashMap<DateTime<Utc>, PathBuf>,
}

// Instantiate the object holding the widgets (views).
pub fn instantiate_ui(app: &Application) -> UserInterface {
    let mut ui = UserInterface {
        settings_file: String::from(SETTINGSFILE),
        win: ApplicationWindow::builder()
            .application(app)
            .title(PROGRAM_NAME)
            .build(),
        header_bar: HeaderBar::builder()
            .title_widget(&WindowTitle::new(PROGRAM_NAME, ""))
            .build(),
        menu_button: MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .build(),
        popover: Popover::builder().build(),
        spinner: Spinner::builder()
            .valign(gtk4::Align::Center)
            .halign(gtk4::Align::Center)
            .visible(false)
            .build(),
        time_widget: DropDown::builder()
            .margin_top(5)
            .margin_bottom(5)
            .margin_start(5)
            .margin_end(5)
            .height_request(30)
            .width_request(100)
            .visible(false)
            .build(),
        status_label: Label::new(Some("")),
        menu_box: gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(10)
            .margin_start(10)
            .margin_end(10)
            .margin_bottom(10)
            .margin_top(10)
            .build(),
        // Main horizontal container to hold the two frames side-by-side,
        // outer box wraps main_pane.
        outer_box: gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(10)
            .build(),
        button_box: gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .vexpand(false)
            .hexpand(false)
            .width_request(200)
            .height_request(20)
            .spacing(10)
            .build(),
        main_pane: gtk4::Paned::builder().build(),
        btn: Button::builder()
            .margin_top(5)
            .margin_bottom(5)
            .margin_start(5)
            .margin_end(5)
            .height_request(30)
            .width_request(50)
            .build(),
        text_view: TextView::builder()
            .monospace(true)
            .editable(false)
            .left_margin(25)
            .right_margin(25)
            .build(),
        text_buffer: TextBuffer::builder().build(),
        frame_left: Frame::builder().margin_bottom(5).build(),
        frame_right: Frame::builder().build(),
        left_frame_pane: gtk4::Paned::builder()
            .orientation(Orientation::Vertical)
            .margin_end(5)
            .shrink_start_child(true)
            .shrink_end_child(true)
            .resize_start_child(true)
            .resize_end_child(true)
            .build(),
        right_frame_pane: gtk4::Paned::builder()
            .orientation(Orientation::Horizontal)
            .margin_start(5)
            .shrink_start_child(true)
            .shrink_end_child(false)
            .resize_start_child(true)
            .resize_end_child(false)
            .build(),
        scrolled_window: ScrolledWindow::builder().margin_top(5).build(),
        da_window: ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .build(),
        curr_time_label: Label::new(Some("")),
        controls_box: gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .width_request(500)
            .spacing(10)
            .build(),
        uom: StringList::new(&[&tr("UNITS_METRIC", None), &tr("UNITS_US", None)]),
        units_widget: DropDown::builder()
            .margin_top(5)
            .margin_bottom(5)
            .margin_start(5)
            .margin_end(5)
            .height_request(30)
            .width_request(100)
            .build(),
        about_label: tr("ABOUT_BUTTON_LABEL", None),
        about_btn: Button::builder()
            .margin_top(5)
            .margin_bottom(5)
            .margin_start(5)
            .margin_end(5)
            .height_request(30)
            .width_request(50)
            .build(),
        da: DrawingArea::builder()
            .width_request(400)
            .margin_end(40)
            .build(),
        lookup: DashMap::new(),
    };
    let provider = gtk4::CssProvider::new();
    let css_data = "textview { font: 14px monospace; font-weight: 500;}";
    provider.load_from_data(css_data);
    gtk4::style_context_add_provider_for_display(
        &gdk::Display::default().expect("Could not get default display."),
        &provider,
        GTK_STYLE_PROVIDER_PRIORITY_APPLICATION.try_into().unwrap(),
    );
    ui.about_btn.set_label(&ui.about_label);
    ui.units_widget.set_model(Some(&ui.uom));
    ui.text_view.set_buffer(Some(&ui.text_buffer));
    ui.text_view
        .set_tooltip_text(Some(&tr("TOOLTIP_TEXT_VIEW", None)));
    ui.scrolled_window.set_child(Some(&ui.text_view));
    ui.about_btn
        .set_tooltip_text(Some(&tr("TOOLTIP_ABOUT_BUTTON", None)));
    ui.menu_box.append(&ui.units_widget);
    ui.menu_box.append(&ui.about_btn);
    ui.popover.set_autohide(true); // Ensures clicking outside or on the button closes it
    ui.popover.set_cascade_popdown(true); // Closes nested popovers if any
    ui.popover.set_child(Some(&ui.menu_box));
    ui.menu_button.set_popover(Some(&ui.popover));
    ui.header_bar.pack_end(&ui.menu_button);
    ui.outer_box.append(&ui.header_bar);
    // Button with icon and label.
    let button_content = gtk4::Box::new(Orientation::Horizontal, 6);
    button_content.set_halign(gtk4::Align::Center);
    // "document-open" is a standard Freedesktop icon name.
    let icon = Image::from_icon_name("document-open");
    let label = Label::new(Some(&tr("SELECT_DIR_TITLE", None)));
    button_content.append(&icon);
    button_content.append(&label);
    ui.btn.set_child(Some(&button_content));
    ui.btn
        .set_tooltip_text(Some(&tr("TOOLTIP_OPEN_BUTTON", None)));

    ui.units_widget
        .set_tooltip_text(Some(&tr("TOOLTIP_UNITS_DROPDOWN", None)));
    ui.win.set_icon_name(Some(ICON_NAME));
    ui.win.set_content(Some(&ui.outer_box));
    // Create the string list used by the time_widget.
    let filtered_variants = get_filtered_variants();
    let labels: Vec<String> = filtered_variants.iter().map(|v| v.get_label()).collect();
    let string_list = gtk4::StringList::new(&labels.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    ui.time_widget.set_model(Some(&string_list));
    ui.button_box.append(&ui.btn);
    ui.button_box.append(&ui.spinner);
    ui.button_box.append(&ui.status_label);
    ui.button_box.append(&ui.time_widget);
    ui.button_box.append(&ui.controls_box);
    ui.outer_box.append(&ui.button_box);
    ui.outer_box.append(&ui.main_pane);
    ui.controls_box.append(&ui.curr_time_label);

    ui.frame_left
        .set_tooltip_text(Some(&tr("TOOLTIP_MAP_FRAME", None)));
    ui.frame_right
        .set_tooltip_text(Some(&tr("TOOLTIP_GRAPH_FRAME", None)));
    // query paths of user-invisible standard directories.
    let base_dirs = BaseDirs::new();
    if base_dirs.is_some() {
        ui.settings_file = base_dirs
            .unwrap()
            .config_dir()
            .join(SETTINGSFILE)
            .to_string_lossy()
            .to_string();
    }
    set_up_user_defaults(&ui);
    return ui;
}
// After reading the fit file, display the additional views of the UI.
pub fn construct_views_from_data(
    ui: &Rc<UserInterface>,
    data: &Vec<(chrono::DateTime<chrono::Utc>, PathBuf)>,
) {
    // 1. Instantiate embedded widgets based on parsed fit data.
    update_map_graph_and_summary_widgets(&ui, &data);

    // 2. Connect embedded widgets to their parents.
    ui.da_window.set_child(Some(&ui.da));
    ui.frame_right.set_child(Some(&ui.da_window));
    ui.frame_left.set_child(Some(&ui.scrolled_window));
    // 3. Configure the widget layout.
    ui.left_frame_pane.set_start_child(Some(&ui.frame_left));
    ui.right_frame_pane.set_start_child(Some(&ui.frame_right));
    // Main box contains all of the above plus the graphs.
    ui.main_pane.set_start_child(Some(&ui.left_frame_pane));
    ui.main_pane.set_end_child(Some(&ui.right_frame_pane));

    // 4. Size the widgets.
    ui.scrolled_window.set_size_request(500, 300);
}

// Connect up the interactive widget handlers.
pub fn connect_interactive_widgets(
    ui: &Rc<UserInterface>,
    data: &Vec<(chrono::DateTime<chrono::Utc>, PathBuf)>,
) {
    // Hook-up the units_widget change handler.
    // update everything when the unit system changes.
    ui.units_widget.connect_selected_notify(clone!(
        #[strong]
        data,
        #[strong]
        ui,
        move |_| {
            update_map_graph_and_summary_widgets(&ui, &data);
            ui.da.queue_draw();
        },
    ));
}
// Return a unit enumeration from a units widget.
pub fn get_unit_system(units_widget: &DropDown) -> Units {
    if units_widget.model().is_some() {
        let model = units_widget.model().unwrap();
        if let Some(item_obj) = model.item(units_widget.selected()) {
            if let Ok(string_obj) = item_obj.downcast::<StringObject>() {
                let unit_string = String::from(string_obj.string());
                if unit_string == tr("UNITS_METRIC", None) {
                    return Units::Metric;
                }
                if unit_string == tr("UNITS_US", None) {
                    return Units::US;
                }
            }
        }
    }
    return Units::None;
}

// Load the application settings from a configuration file.
pub fn set_up_user_defaults(ui: &UserInterface) {
    let config = load_config(&Path::new(&ui.settings_file));
    ui.win.set_default_size(config.width, config.height);
    ui.main_pane.set_position(config.main_split);
    ui.right_frame_pane.set_position(config.right_frame_split);
    ui.left_frame_pane.set_position(config.left_frame_split);
    ui.units_widget.set_selected(config.units_index);
}

pub fn get_selected_start_end(ui: &UserInterface) -> (DateTime<Utc>, DateTime<Utc>) {
    let index = ui.time_widget.selected() as usize;
    let filtered_variants = get_filtered_variants();
    if let Some(selected_variant) = filtered_variants.get(index) {
        let (start, end) = get_time_range(selected_variant.clone());
        return (start, end);
    }
    return (Utc::now(), Utc::now());
}

// #####################################################################
// ##################### GRAPH FUNCTIONS ###############################
// #####################################################################
//
//
// Perform this ONCE in main.rs
pub fn collect_all_stats(results: &[(DateTime<Utc>, PathBuf)]) -> Vec<PlottableData> {
    results
        .into_par_iter()
        .map(|(ts, path)| PlottableData {
            timestamp: *ts,
            stats: extract_session_data(path).unwrap_or_default(),
        })
        .collect()
}

pub fn convert_all_stats(raw_stats: &Vec<PlottableData>, ui: &UserInterface) -> Vec<PlottableData> {
    let selected_units = get_unit_system(&ui.units_widget);
    raw_stats
        .into_par_iter()
        .map(|plottable_data| PlottableData {
            timestamp: plottable_data.timestamp,
            stats: convert_session_data(&plottable_data.stats, &selected_units).unwrap_or_default(),
        })
        .collect()
}

// Convert the above structure to plottable vectors
pub fn get_metric_vec(
    all_data: &[PlottableData],
    value_extractor: fn(&SessionStats) -> f64,
) -> Vec<(DateTime<Utc>, f64)> {
    let mut data: Vec<(DateTime<Utc>, f64)> = all_data
        .iter()
        .map(|item| (item.timestamp, value_extractor(&item.stats)))
        .collect();

    // Ensure chronological order for the LineSeries
    data.sort_by_key(|(ts, _)| *ts);
    data
}

/// Generates a LineSeries chart for a specific metric.
pub fn build_individual_graph(
    ui: &UserInterface,
    a: &plotters::drawing::DrawingArea<CairoBackend<'_>, plotters::coord::Shift>,
    // results: &[(DateTime<Utc>, PathBuf)],
    plotvals: Vec<(DateTime<Utc>, f64)>,
    metric_name: &str,
    unit_label: &str,
    color: &RGBColor,
) -> Result<(), Box<dyn std::error::Error>> {
    if plotvals.is_empty() {
        return Ok(());
    }

    let (start_date, end_date) = get_selected_start_end(&ui);
    let max_val = plotvals.iter().map(|(_, v)| *v).fold(0.0, f64::max) * 1.1;

    let is_dark = StyleManager::default().is_dark();
    let mut caption_style = ("sans-serif", 16, &GREY_800).into_text_style(a);
    if is_dark {
        caption_style = ("sans-serif", 16, &GREY_200).into_text_style(a);
    }
    let mut chart = ChartBuilder::on(&a)
        .caption(format!("{}", metric_name), caption_style)
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(start_date..end_date, 0.0..max_val)?;

    let mut axis_text_style = ("sans-serif", 10, &GREY_800).into_text_style(a);
    if is_dark {
        axis_text_style = ("sans-serif", 10, &GREY_200).into_text_style(a);
    }
    let light_line_style = ShapeStyle {
        color: color.mix(0.05),
        filled: false,
        stroke_width: 1,
    };
    let bold_line_style = ShapeStyle {
        color: color.mix(0.10),
        filled: false,
        stroke_width: 2,
    };
    let mut axis_style = ShapeStyle {
        color: GREY_600.mix(1.0),
        filled: false,
        stroke_width: 2,
    };
    if is_dark {
        axis_style = ShapeStyle {
            // color: color.mix(0.3),
            color: GREY_400.mix(1.0),
            filled: false,
            stroke_width: 2,
        };
    }

    chart
        .configure_mesh()
        .x_labels(16)
        .x_label_style(axis_text_style.clone())
        .y_labels(5)
        .y_label_style(axis_text_style.clone())
        .x_label_formatter(&|d| d.format("%m-%d").to_string())
        .y_desc(unit_label)
        .light_line_style(light_line_style)
        .bold_line_style(bold_line_style)
        .axis_style(axis_style)
        .draw()?;

    chart.draw_series(LineSeries::new(
        plotvals.iter().map(|(date, val)| (*date, *val)),
        &color,
    ))?;

    chart.draw_series(
        plotvals
            .iter()
            .map(|(date, val)| Circle::new((*date, *val), 3, color.filled())),
    )?;

    Ok(())
}
// Use plotters.rs to draw a graph on the drawing area.
fn draw_graphs(
    ui: &UserInterface,
    distance_plotvals: &Vec<(DateTime<Utc>, f64)>,
    calories_plotvals: &Vec<(DateTime<Utc>, f64)>,
    ascent_plotvals: &Vec<(DateTime<Utc>, f64)>,
    duration_plotvals: &Vec<(DateTime<Utc>, f64)>,
    pace_plotvals: &Vec<(DateTime<Utc>, f64)>,
    descent_plotvals: &Vec<(DateTime<Utc>, f64)>,
    cr: &Context,
    width: f64,
    height: f64,
) {
    let selected_units = get_unit_system(&ui.units_widget);
    let root = plotters_cairo::CairoBackend::new(&cr, (width as u32, height as u32))
        .unwrap()
        .into_drawing_area();
    let areas = root.split_evenly((3, 2));

    let mut distance_unit = "";
    let mut speed_unit = "";
    let mut ascent_unit = "";
    let mut descent_unit = "";
    match selected_units {
        Units::Metric => {
            distance_unit = "Kilometers";
            speed_unit = "Minutes/Km";
            ascent_unit = "Meters";
            descent_unit = "Meters";
        }
        Units::US => {
            distance_unit = "Miles";
            speed_unit = "Minutes/Mile";
            ascent_unit = "Feet";
            descent_unit = "Feet";
        }
        _ => {}
    }

    build_individual_graph(
        &ui,
        &areas[0],
        distance_plotvals.to_vec(),
        "Distance",
        distance_unit,
        &GREEN,
    )
    .unwrap();
    build_individual_graph(
        &ui,
        &areas[1],
        calories_plotvals.to_vec(),
        "Calories",
        "Kcal",
        &BLUE,
    )
    .unwrap();
    build_individual_graph(
        &ui,
        &areas[2],
        pace_plotvals.to_vec(),
        "Pace",
        speed_unit,
        &BROWN,
    )
    .unwrap();
    build_individual_graph(
        &ui,
        &areas[3],
        duration_plotvals.to_vec(),
        "Duration",
        "Minutes",
        &RED,
    )
    .unwrap();
    build_individual_graph(
        &ui,
        &areas[4],
        ascent_plotvals.to_vec(),
        "Elevation Gain",
        ascent_unit,
        &CYAN,
    )
    .unwrap();
    build_individual_graph(
        &ui,
        &areas[5],
        descent_plotvals.to_vec(),
        "Elevation Loss",
        descent_unit,
        &YELLOW,
    )
    .unwrap();

    let _ = root.present();
}

// Build the graphs.  Prepare the graphical data for the drawing area and
// set-up the draw function callback.
fn build_graphs(stats: &Vec<PlottableData>, ui: &Rc<UserInterface>) {
    // Need to clone to use inside the closure.
    let distance_plotvals: Vec<(DateTime<Utc>, f64)> =
        get_metric_vec(&stats, |s| s.distance as f64);
    let calories_plotvals: Vec<(DateTime<Utc>, f64)> =
        get_metric_vec(&stats, |s| s.calories as f64);
    let ascent_plotvals: Vec<(DateTime<Utc>, f64)> = get_metric_vec(&stats, |s| s.ascent as f64);
    let duration_plotvals: Vec<(DateTime<Utc>, f64)> =
        get_metric_vec(&stats, |s| s.duration as f64);
    let pace_plotvals: Vec<(DateTime<Utc>, f64)> =
        get_metric_vec(&stats, |s| s.enhanced_speed as f64);
    let descent_plotvals: Vec<(DateTime<Utc>, f64)> = get_metric_vec(&stats, |s| s.descent as f64);
    ui.da.set_draw_func(clone!(
        #[strong]
        ui,
        move |_drawing_area, cr, width, height| {
            draw_graphs(
                &ui,
                &distance_plotvals,
                &calories_plotvals,
                &ascent_plotvals,
                &duration_plotvals,
                &pace_plotvals,
                &descent_plotvals,
                cr,
                width as f64,
                height as f64,
            );
        }
    ));
}

// Update the views when supplied with data.
fn update_map_graph_and_summary_widgets(
    ui: &Rc<UserInterface>,
    data: &Vec<(chrono::DateTime<chrono::Utc>, PathBuf)>,
) {
    let stats = collect_all_stats(data);
    // units conversion
    let ui_stats = convert_all_stats(&stats, &ui);
    build_graphs(&ui_stats, &ui);
    build_summary(&ui_stats, &ui);
    return;
}

// #####################################################################
// ##################### SUMMARY FUNCTIONS #############################
// #####################################################################
// Build a summary using the PlottableData struct
fn build_summary(stat_collection: &Vec<PlottableData>, ui: &UserInterface) {
    // 1. Clear out the existing buffer
    let mut start = ui.text_buffer.start_iter();
    let mut end = ui.text_buffer.end_iter();
    ui.text_buffer.delete(&mut start, &mut end);

    // 2. Insert Table Header
    let mut header = format!("");
    let selected_units = get_unit_system(&ui.units_widget);
    match selected_units {
        Units::Metric => {
            header = format!(
                "{:<18} | {:<9} | {:<12} | {:<11} | {:<14} | {:<7} | {:<7}\n",
                "Date & Time",
                "Dist(km)",
                "Cal (kcal)",
                "Time(min)",
                "Pace(min/km)  ",
                "Asc(m)",
                "Des(m)"
            );
        }
        Units::US => {
            header = format!(
                "{:<18} | {:<9} | {:<12} | {:<11} | {:<14} | {:<7} | {:<7}\n",
                "Date & Time",
                "Dist(mi)",
                "Cal (kcal)",
                "Time(min)",
                "Pace(min/mile)",
                "Asc(ft)",
                "Des(ft)"
            );
        }
        _ => {}
    }

    ui.text_buffer.insert(&mut end, &header);
    ui.text_buffer.insert(&mut end, &format!("{:-<96}\n", ""));

    // 3. Collect all stats into the PlottableData struct (Parse Once)
    let mut plottable_collection = stat_collection.clone();

    // 4. Sort by timestamp to ensure chronological order in the text view
    plottable_collection.sort_by_key(|item| item.timestamp);

    // 5. Iterate through the collection and format the strings

    let pace_formatter = |x: &f32| {
        let mins = x.trunc();
        let secs = x.fract() * 60.0;
        format!("{:02.0}:{:02.0}", mins, secs)
    };
    for item in plottable_collection {
        let ts = item.timestamp;
        let stats = item.stats;

        let row = format!(
            "{:<18} | {:>9.2} | {:>12} | {:>11.1} | {:>14} | {:>7.0} | {:>7.0}\n",
            ts.format("%Y-%m-%d").to_string(),
            stats.distance,
            stats.calories,
            stats.duration,
            pace_formatter(&(stats.enhanced_speed as f32)),
            stats.ascent,
            stats.descent
        );

        // Re-calculate end iterator to ensure we append to the bottom
        let mut end_iter = ui.text_buffer.end_iter();
        ui.text_buffer.insert(&mut end_iter, &row);
    }
}
