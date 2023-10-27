mod hex_app;
use egui::vec2;
use eframe::NativeOptions;
use egui_notify::Toasts;
use hex_app::HexApp;

fn main() {
    let options: NativeOptions = NativeOptions {
        always_on_top: false,
        centered: true,
        drag_and_drop_support: true,
        initial_window_size: Some(vec2(1000.0, 1000.0)),
        ..Default::default()
    };

    let mut app = Box::<HexApp>::default();
    app.column_count = 32;

    app.toasts = Toasts::default();

    if let Err(err) = eframe::run_native(
        "HexView",
        options,
        Box::new(|_| app)
    ) {
        eprintln!("{}", err);
    }
}
