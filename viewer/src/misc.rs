use eframe::egui;

pub fn ui_callsite(ui: &mut egui::Ui, callsite: &rr_data::Callsite) {
    let rr_data::Callsite {
        id,
        kind,
        name,
        level,
        location,
        field_names,
    } = callsite;

    use itertools::Itertools as _;

    egui::Grid::new("callsite")
        .spacing([8.0, 2.0])
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            ui.label("Name:");
            ui.label(format!("{:?}", name.as_str()));
            ui.end_row();

            ui.label("Id:");
            ui.label(id.to_string());
            ui.end_row();

            ui.label("Kind:");
            ui.label(kind.to_string());
            ui.end_row();

            ui.label("Level:");
            ui.label(level.to_string());
            ui.end_row();

            ui.label("Location:");
            ui.label(location.to_string());
            ui.end_row();

            ui.label("Field names:");
            ui.label(field_names.iter().join(" "));
            ui.end_row();
        });
}

pub fn format_time(time: &rr_data::Time) -> String {
    let nanos_since_epoch = time.nanos_since_epoch();
    let years_since_epoch = nanos_since_epoch / 1_000_000_000 / 60 / 60 / 24 / 365;
    if 50 <= years_since_epoch && years_since_epoch <= 150 {
        use chrono::TimeZone as _;
        let datetime = chrono::Utc.timestamp(
            nanos_since_epoch / 1_000_000_000,
            (nanos_since_epoch % 1_000_000_000) as _,
        );

        if datetime.date() == chrono::offset::Utc::today() {
            datetime.format("%H:%M:%S%.3fZ").to_string()
        } else {
            datetime.format("%Y-%m-%d %H:%M:%S%.3fZ").to_string()
        }
    } else {
        let secs = nanos_since_epoch as f64 * 1e-9;
        // assume relative time
        format!("+{:.03}s", secs)
    }
}
