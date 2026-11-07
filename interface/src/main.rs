use std::{
    sync::{Arc, Mutex},
    thread,
};

use anyhow::Result;
use clone_macro::clone;
use eframe::NativeOptions;
use egui::CentralPanel;
use sartorius_competence::{Scale, WeighResult};

struct ScaleState {
    weight: Mutex<WeighResult>,
}

fn main() -> Result<()> {
    let mut scale = Scale::connect_default("/dev/ttyUSB0")?;
    let (model, serial) = (scale.model().to_owned(), scale.cell_serial().to_owned());
    let state = Arc::new(ScaleState {
        weight: Mutex::new(WeighResult::Preview { value: 0.0 }),
    });

    thread::spawn(clone!([state], move || {
        loop {
            if let Ok(result) = scale.weigh() {
                *state.weight.lock().unwrap() = result;
            }
        }
    }));

    eframe::run_ui_native(
        "Sartorius Interface",
        NativeOptions::default(),
        move |ui, _frame| {
            CentralPanel::default_margins().show(ui, |ui| {
                ui.request_repaint();
                ui.label(format!("Connected to {model} ({serial})."));

                let weight = state.weight.lock().unwrap().clone();
                ui.vertical(|ui| {
                    match weight {
                        WeighResult::Preview { value } => {
                            ui.heading(value.to_string());
                            ui.weak("PREVIEW");
                        }
                        WeighResult::Weight { value, unit } => {
                            ui.heading(format!("{value} {}", unit.symbol()));
                            ui.weak("STABLE");
                        }
                    };
                });
            });
        },
    )?;
    Ok(())
}
