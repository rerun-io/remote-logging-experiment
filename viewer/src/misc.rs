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
