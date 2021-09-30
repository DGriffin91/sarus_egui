use std::{cell::RefCell, ffi::CStr, mem, rc::Rc};

use dynfmt::{Format, SimpleCurlyFormat};
use eframe::{
    egui::{self, FontDefinitions, FontFamily, Ui},
    epi,
};
use sarus::{default_std_jit_from_code, jit::JIT, parser, sarus_std_lib};

use crate::highligher::MemoizedSyntaxHighlighter;

extern "C" fn label(ui: &mut Ui, s: *const i8) {
    let s = unsafe { CStr::from_ptr(s).to_str().unwrap() };
    ui.label(s);
}

extern "C" fn button(ui: &mut Ui, s: *const i8) -> bool {
    let s = unsafe { CStr::from_ptr(s).to_str().unwrap() };
    ui.button(s).clicked()
}

extern "C" fn slider(ui: &mut Ui, s: *const i8, x: f64, range_btm: f64, range_top: f64) -> f64 {
    let s = unsafe { CStr::from_ptr(s).to_str().unwrap() };
    let mut slider_f32 = x as f32;
    ui.add(egui::Slider::new(&mut slider_f32, range_btm as f32..=range_top as f32).text(s));
    slider_f32 as f64
}

const DEFAULT_CODE: &str = r#"

struct Ui {
    ui: &,
}

extern fn label(self: Ui, str: &) -> () {}
extern fn button(self: Ui, str: &) -> (pressed: bool) {}
extern fn slider(self: Ui, s: &, x: f64, range_btm: f64, range_top: f64) -> (y: f64) {}

fn f_to_c(f: f64) -> (c: f64) {
    c = (f - 32.0) * 5.0/9.0
}

fn c_to_f(c: f64) -> (f: f64) {
    f = (c * 9.0/5.0) + 32.0
}

fn main(ui: Ui, x: &[f64]) -> () {
    ui.label("Celsius / Fahrenheit converter")
    x[0] = ui.slider("Celsius", x[0], -200.0, 300.0)
    x[0] = f_to_c(ui.slider("Fahrenheit", c_to_f(x[0]), -200.0, 300.0))
    if ui.button("freezing") {
        x[0] = 0.0
    }
    if ui.button("boiling") {
        x[0] = 100.0
    }
}

"#;

fn compile(code: &str) -> anyhow::Result<JIT> {
    let mut jit = default_std_jit_from_code(
        code,
        Some(vec![
            ("Ui.label", label as *const u8),
            ("Ui.button", button as *const u8),
            ("Ui.slider", slider as *const u8),
        ]),
    )?;
    Ok(jit)
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
pub struct SarusEgui {
    // Example stuff:
    label: String,

    // this how you opt-out of serialization of a member
    #[cfg_attr(feature = "persistence", serde(skip))]
    values: [f64; 4],
    func: Option<extern "C" fn(&mut Ui, &mut [f64; 4])>,
    code: String,
    errors: String,
    highlighter: MemoizedSyntaxHighlighter,
}

impl Default for SarusEgui {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            values: [0f64; 4],
            func: None,
            code: DEFAULT_CODE.to_owned(),
            errors: String::new(),
            highlighter: Default::default(),
        }
    }
}

impl epi::App for SarusEgui {
    fn name(&self) -> &str {
        "egui template"
    }

    /// Called once before the first frame.
    fn setup(
        &mut self,
        ctx: &egui::CtxRef,
        _frame: &mut epi::Frame<'_>,
        _storage: Option<&dyn epi::Storage>,
    ) {
        let mut fonts = FontDefinitions::default();

        fonts.font_data.insert(
            "FiraCode".to_owned(),
            std::borrow::Cow::Borrowed(include_bytes!("../FiraCode-Regular.ttf")),
        ); // .ttf and .otf supported

        fonts
            .fonts_for_family
            .get_mut(&FontFamily::Monospace)
            .unwrap()[0] = "FiraCode".to_owned();

        for (_text_style, (_family, size)) in fonts.family_and_size.iter_mut() {
            *size = 25.0;
        }
        ctx.set_fonts(fonts);

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        #[cfg(feature = "persistence")]
        if let Some(storage) = _storage {
            *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
        }
    }

    /// Called by the frame work to save state before shutdown.
    /// Note that you must enable the `persistence` feature for this to work.
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        let Self {
            label,
            values,
            func,
            code,
            errors,
            highlighter,
        } = self;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                egui::menu::menu(ui, "File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                });
            });
        });

        egui::SidePanel::left("side_panel")
            .min_width(400.0)
            .show(ctx, |ui| {
                if let Some(func) = func {
                    func(ui, values);
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Compile").clicked() {
                *errors = String::from("");
                *func = match compile(&code.replace("\r\n", "\n")) {
                    Ok(mut jit) => match jit.get_func("main") {
                        Ok(func_ptr) => unsafe {
                            Some(mem::transmute::<
                                _,
                                extern "C" fn(ui: &mut Ui, x: &mut [f64; 4]),
                            >(func_ptr))
                        },
                        Err(e) => {
                            *errors = e.to_string();
                            None
                        }
                    },
                    Err(e) => {
                        *errors = e.to_string();
                        None
                    }
                }
            }
            let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
                let mut layout_job =
                    highlighter.highlight(ui.visuals().dark_mode, string, "rs".into());
                layout_job.wrap_width = wrap_width;
                ui.fonts().layout_job(layout_job)
            };

            ui.visuals_mut().extreme_bg_color = egui::Color32::from_rgb(39, 40, 34);
            ui.add(
                egui::TextEdit::multiline(code)
                    .desired_width(f32::INFINITY)
                    .lock_focus(true)
                    .text_style(egui::TextStyle::Monospace)
                    .layouter(&mut layouter), // for cursor height
            );
            ui.visuals_mut().extreme_bg_color = egui::Color32::from_rgb(20, 20, 20);
            ui.add(
                egui::TextEdit::multiline(errors)
                    .desired_width(f32::INFINITY)
                    .text_style(egui::TextStyle::Monospace), // for cursor height
            );
        });
    }

    fn warm_up_enabled(&self) -> bool {
        false
    }

    fn on_exit(&mut self) {}

    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(30)
    }

    fn max_size_points(&self) -> egui::Vec2 {
        egui::Vec2::new(2160.0, 3840.0)
    }

    fn clear_color(&self) -> egui::Rgba {
        egui::Color32::from_rgba_unmultiplied(2, 2, 2, 180).into()
    }
}
