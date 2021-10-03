use std::ffi::CStr;

use eframe::egui;
use eframe::egui::Ui;
use sarus::decl;
use sarus::frontend::Arg;
use sarus::frontend::Declaration;
use sarus::frontend::Function;

use sarus::jit::JITBuilder;
use sarus::validator::ExprType as E;

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

#[rustfmt::skip]
pub fn append_egui(
    prog: &mut Vec<Declaration>,
    jit_builder: &mut JITBuilder,
) {
    let jb = jit_builder;
    decl!(prog, jb, "Ui.label",     label,     (E::Struct(Box::new("Ui".to_string())),E::Address),                      ());
    decl!(prog, jb, "Ui.button",    button,    (E::Struct(Box::new("Ui".to_string())),E::Address),                      (E::Bool));
    decl!(prog, jb, "Ui.slider",    slider,    (E::Struct(Box::new("Ui".to_string())),E::Address,E::F64,E::F64,E::F64), (E::F64));
}
