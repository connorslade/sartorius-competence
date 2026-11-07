use std::{
    mem,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

use anyhow::Result;
use clone_macro::clone;
use eframe::NativeOptions;
use egui::{Button, CentralPanel, ViewportBuilder};
use sartorius_competence::{Scale, WeighResult};

struct ScaleState {
    weight: Mutex<WeighResult>,
    tare: AtomicBool,
}

enum Queue {
    None,
    Copy,
}

fn main() -> Result<()> {
    let mut scale = Scale::connect_default("/dev/ttyUSB0")?;
    let (model, serial) = (scale.model().to_owned(), scale.cell_serial().to_owned());

    let mut queue = Queue::None;
    let state = Arc::new(ScaleState {
        weight: Mutex::new(WeighResult::Preview { value: 0.0 }),
        tare: AtomicBool::new(false),
    });

    thread::spawn(clone!([state], move || {
        loop {
            if state.tare.load(Ordering::Relaxed) && scale.tare().is_ok() {
                state.tare.store(false, Ordering::Relaxed);
            }

            if let Ok(result) = scale.weigh() {
                *state.weight.lock().unwrap() = result;
            }
        }
    }));

    eframe::run_ui_native(
        "Sartorius Interface",
        NativeOptions {
            viewport: ViewportBuilder::default().with_inner_size([210.0, 97.0]),
            ..Default::default()
        },
        move |ui, _frame| {
            CentralPanel::default_margins().show(ui, |ui| {
                ui.request_repaint();
                ui.label(format!("Connected to {model} ({serial})."));

                let weight = state.weight.lock().unwrap().clone();
                match weight {
                    WeighResult::Preview { value } => {
                        ui.heading(value.to_string());
                        ui.weak("PREVIEW");
                    }
                    WeighResult::Weight { value, unit } => {
                        let weight = format!("{value} {}", unit.symbol());
                        ui.heading(&weight);
                        ui.weak("STABLE");

                        match mem::replace(&mut queue, Queue::None) {
                            Queue::None => {}
                            Queue::Copy => ui.copy_text(weight),
                        }
                        queue = Queue::None;
                    }
                };

                ui.horizontal(|ui| {
                    let tare = state.tare.load(Ordering::Relaxed);
                    if ui.add_enabled(!tare, Button::new("Tare")).clicked() {
                        state.tare.store(true, Ordering::Relaxed);
                    }

                    let queue_empty = matches!(queue, Queue::None);

                    if ui.add_enabled(queue_empty, Button::new("Copy")).clicked() {
                        queue = Queue::Copy;
                    }
                });
            });
        },
    )?;
    Ok(())
}
