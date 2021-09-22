// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use eframe::egui::Vec2;

    let app = sarus_egui::SarusEgui::default();
    let mut native_options = eframe::NativeOptions::default();
    native_options.initial_window_size = Some(Vec2 {
        x: 1800.0,
        y: 1024.0,
    });
    eframe::run_native(Box::new(app), native_options);
}
