use fluent::{FluentArgs, FluentBundle, FluentResource};
use std::cell::RefCell;
use unic_langid::LanguageIdentifier;

thread_local! {
    static BUNDLE: RefCell<FluentBundle<FluentResource>> = RefCell::new({
    let mut locale = sys_locale::get_locale().unwrap_or_else(|| String::from("en-US"));
    // Debian (or Gnome) sets this rather bogus default on the testbed. We'll work around.
    if locale == "C.utf8" || locale == "C" {locale = String::from("en-US")};
    let lang_id: LanguageIdentifier = locale.parse().expect("Parsing failed");

    let mut bundle = FluentBundle::new(vec![lang_id.clone()]);

    // Load the appropriate file based on the detected language
    let ftl_content = match lang_id.language.as_str() {
        "es" => include_str!("../i18n/es/gui_aggregate.ftl"),
        "fr" => include_str!("../i18n/fr/gui_aggregate.ftl"),
        _ => include_str!("../i18n/en-US/gui_aggregate.ftl"),
    };

    let resource = FluentResource::try_new(ftl_content.to_string())
        .expect("Failed to parse FTL");
    bundle.add_resource(resource).expect("Failed to add resource");

        bundle
    });
}

pub fn tr(msg_id: &str, args: Option<&FluentArgs>) -> String {
    BUNDLE.with(|bundle_cell| {
        let bundle = bundle_cell.borrow();
        let msg = bundle.get_message(msg_id).expect("Message not found");
        let pattern = msg.value().expect("Message has no value");
        let mut errors = vec![];
        bundle
            .format_pattern(pattern, args, &mut errors)
            .to_string()
    })
}
