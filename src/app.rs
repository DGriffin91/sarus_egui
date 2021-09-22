use std::{ffi::CStr, mem};

use dynfmt::{Format, SimpleCurlyFormat};
use eframe::{
    egui::{self, FontDefinitions, FontFamily, Ui},
    epi,
};
use sarus::{
    jit::{self, JIT},
    parser, sarus_std_lib,
};
extern "C" fn format(ret: *mut i8, format: *const i8, x: f64) -> *const i8 {
    let s = unsafe { CStr::from_ptr(format).to_str().unwrap() };
    let formatted = SimpleCurlyFormat.format(s, &[x]).unwrap();
    let bytes = formatted.as_bytes();
    for (i, c) in bytes.iter().enumerate() {
        unsafe {
            *ret.offset(i as isize) = *c as i8;
        }
    }
    unsafe {
        *ret.offset(bytes.len() as isize) = '\0' as i8;
    }

    ret
}

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

struct MyObject {
    pub v: Vec<f64>,
}

impl Drop for MyObject {
    fn drop(&mut self) {
        //println!("dropping MyObject");
    }
}

extern "C" fn f64vec() -> *mut MyObject {
    Box::into_raw(Box::new(MyObject { v: Vec::new() }))
}

extern "C" fn f64vec_push(vec: *mut MyObject, val: f64) {
    let vec = unsafe {
        assert!(!vec.is_null());
        &mut *vec
    };
    vec.v.push(val);
}

extern "C" fn f64vec_get(vec: *const MyObject, idx: i64) -> f64 {
    let vec = unsafe {
        assert!(!vec.is_null());
        &*vec
    };
    vec.v[idx as usize]
}

extern "C" fn f64vec_drop(vec: *mut MyObject) {
    unsafe {
        assert!(!vec.is_null());
        Box::from_raw(vec)
    };
}

const DEFAULT_CODE: &str = r#"
extern fn label(ui: &, str: &) -> () {}
extern fn button(ui: &, str: &) -> (pressed: bool) {}
extern fn slider(ui: &, s: &, x: f64, range_btm: f64, range_top: f64) -> (y: f64) {}
extern fn format(ret: &, format: &, x: f64) -> (r: &) {}
extern fn f64vec() -> (r: &) {}
extern fn f64vec_push(vec: &, val: f64) -> () {}
extern fn f64vec_get(vec: &, idx: i64) -> (r: f64) {}
extern fn f64vec_drop(vec: &) -> () {}

fn main(ui: &, x: &[f64]) -> () {
    label(ui, "HELLO")
    if button(ui, "increment") {
        x[0] = x[0] + 1.0
    }
    if button(ui, "decrement") {
        x[0] = x[0] - 1.0
    }
    label(ui, "HELLO")
    label(ui, format([" "; 100], "Value: {}", x[0]))
    if button(ui, "decrement by 10") {
        x[0] = x[0] - 10.0
    }
    x[0] = slider(ui, "Slider", x[0], 0.0, 100.0)
    vec = f64vec()
    f64vec_push(vec, x[0])
    f64vec_push(vec, x[0] * 10.0)
    label(ui, format([" "; 100], "Vec Value 0: {}", f64vec_get(vec, 0)))
    label(ui, format([" "; 100], "Vec Value 1: {}", f64vec_get(vec, 1)))
    f64vec_drop(vec)
}

"#;

fn compile(code: &str) -> anyhow::Result<JIT> {
    let mut jit = jit::JIT::new(&[
        ("label", label as *const u8),
        ("button", button as *const u8),
        ("format", format as *const u8),
        ("f64vec", f64vec as *const u8),
        ("f64vec_push", f64vec_push as *const u8),
        ("f64vec_get", f64vec_get as *const u8),
        ("f64vec_drop", f64vec_drop as *const u8),
        ("slider", slider as *const u8),
    ]);
    let ast = parser::program(code)?;
    let ast = sarus_std_lib::append_std_funcs(ast);
    jit.translate(ast.clone())?;
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
                *func = match compile(&code) {
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
            ui.add(
                egui::TextEdit::multiline(code)
                    .desired_width(f32::INFINITY)
                    .lock_focus(true)
                    .text_style(egui::TextStyle::Monospace), // for cursor height
            );
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