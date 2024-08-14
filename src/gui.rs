use std::{sync::mpsc::Receiver, time::Duration};

use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};

use crate::audio::{Chunk, BUFSIZE};

struct MyApp {
    name: String,
    age: u32,
    rx: Receiver<Chunk>
}

impl MyApp {
    fn new(rx: Receiver<Chunk>) -> Self {
        Self {name: "Bruh".to_owned(), age: 11, rx}
    }
}

// impl Default for MyApp {
//     fn default() -> Self {
//         Self {
//             name: "Arthur".to_owned(),
//             age: 42,
//         }
//     }
// }

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // let sin: PlotPoints = (0..1000).map(|i| {
        //     let x = i as f64 * 0.01;
        //     [x, x.sin()]
        // }).collect();
        let x = match self.rx.recv(){
            Ok(chunk) => chunk,
            Err(_) => [0.0; BUFSIZE],
        };
        let sin: PlotPoints = x.into_iter().enumerate().map(|(n, val)| [n as f64, val as f64]).collect();
        let line = Line::new(sin);
        let my_plot = Plot::new("my_plot").view_aspect(2.0);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Application");
            ui.horizontal(|ui| {
                let name_label = ui.label("Your name: ");
                ui.text_edit_singleline(&mut self.name)
                    .labelled_by(name_label.id);
            });
            ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
            if ui.button("Increment").clicked() {
                self.age += 1;
            }
            ui.label(format!("Hello '{}', age {}", self.name, self.age));

            my_plot.show(ui, |plot_ui| plot_ui.line(line));
        });

        ctx.request_repaint_after(Duration::from_millis(50));
    }
}

pub(crate) fn start_gui(rx: Receiver<[f32; 512]>) -> Result<(), eframe::Error>{
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "My egui App",
        options,
        Box::new(move |_cc| {
            let app = Box::new(MyApp::new(rx));
            Ok(app)
        }),
    )
}