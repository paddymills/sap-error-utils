
use std::fs::{File, self};
use std::io::{self, Write};

use eframe::{self, egui};

use crate::inbox::parsers::parse_failures;
use crate::paths;

const MAX_FILES: usize = 2000;
const INPUT_FILENAME: &str = "inbox.txt";
const PARTS_FILENAME: &str = "parts.txt";

#[derive(Debug,Default)]
pub struct SapInboxApp {
    reset: bool,
    files_to_parse: usize,

    status: String,
}

impl SapInboxApp {
    const NAME: &str = "SAP Inbox Errors";

    pub fn new() -> eframe::AppCreator {
        Box::new(|cc| Box::new(Self::init(cc)))
    }

    pub fn run() -> eframe::Result<()> {
        eframe::run_native(Self::NAME, Self::win_opts(), Self::new())
    }

    fn win_opts() -> eframe::NativeOptions {
        eframe::NativeOptions {
            centered: true,
            default_theme: eframe::Theme::Light,

            max_window_size: Some([300., 200.].into()),

            ..Default::default()
        }
    }

    fn init(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            files_to_parse: 200,

            ..Default::default()
        }
    }

    fn generate_parts(&self) -> io::Result<()> {
        let file = std::path::PathBuf::from(INPUT_FILENAME);
        let inbox = parse_failures(file)?;

        // get marks only from failures
        let mut marks: Vec<&String> = inbox
            .iter()
            .map(|f| &f.mark)
            .collect();

        // remove duplicates
        marks.sort();
        marks.dedup();

        let mut buffer = File::create(PARTS_FILENAME)?;
        for m in marks {
            writeln!(buffer, "{}", m)?;
        }

        Ok(())
    }

    fn move_prodfiles(&self) -> io::Result<()> {

        for entry in glob::glob("Production_*.ready").unwrap() {
            match entry {
                Ok(prodfile) => {
                    let mut to = paths::SAP_OUTBOUND.to_path_buf();
                    to.push(&prodfile);

                    fs::copy(&prodfile, to)?;
                    fs::remove_file(&prodfile)?;
                },
                Err(_) => todo!("handle error")
            }
        }


        Ok(())
    }
}

impl eframe::App for SapInboxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::bottom("<footer>")
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(&self.status);
                });
            });

        egui::CentralPanel::default()
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    if ui.button("Genereate parts list").clicked() {
                        // TODO: log failure
                        let status = match self.generate_parts() {
                            Ok(_) => "parts list generated".into(),
                            Err(e) => format!("Error generating partslist: {}", e)
                        };
                    }
                    if ui.button("Genereate confirmation file").clicked() {
                        self.status = "confirmation file generated".into();
                    }
                    if ui.button("Move confirmation file(s)").clicked() {
                        self.status = match self.move_prodfiles() {
                            Ok(_) => "file(s) moved".into(),
                            Err(e) => format!("Error moving files: {}", e)
                        };
                    }

                    ui.group(|ui| {

                        ui.checkbox(&mut self.reset, "Remove generated files");
                        
                        ui.horizontal_centered(|ui| {
                            ui.label("Files to search");
                            ui.add(
                                // TODO: get total number a file to make a max
                                egui::DragValue::new(&mut self.files_to_parse)
                                    .speed(10.0)
                                    .clamp_range(10..=MAX_FILES)
                                    .custom_formatter(|n, _| {
                                        if n == MAX_FILES as f64 {
                                            return String::from("all");
                                        }

                                        format!("{n}")
                                    })
                            );
                        });
                    });

                    // TODO: fake terminal for logging
                });
            });
    }
}
