
use std::fs::{File, self};
use std::io::{self, Write};
use std::path::PathBuf;

use eframe::{self, egui};

use crate::api::{Order, CnfFileRow};
use crate::inbox::FailureMatchStatus;
use crate::inbox::parsers::{parse_failures, parse_cohv_xl};
use crate::inbox::cnf_files::{self, get_last_n_files, parse_file, write_file};
use crate::paths;

const MAX_FILES: usize = 2000;
const INPUT_FILENAME: &str = "inbox.txt";
const PARTS_FILENAME: &str = "parts.txt";

#[derive(Debug,Default)]
pub struct SapInboxApp {
    files_to_parse: usize,
    max_files: usize,
    auto_move_files: bool,

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

            ..Default::default()
        }
    }

    fn init(cc: &eframe::CreationContext<'_>) -> Self {
        let auto_move = match cc.storage {
            Some(storage) => storage.get_string("auto_move").unwrap_or_default() == "true",
            None => false
        };

        Self {
            files_to_parse: 200,
            max_files: cnf_files::get_num_files().unwrap_or(MAX_FILES),
            auto_move_files: auto_move,

            ..Default::default()
        }
    }

    pub fn generate_parts(&self) -> io::Result<()> {
        let file = PathBuf::from(INPUT_FILENAME);
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

    pub fn generate_comparison(&mut self) -> anyhow::Result<()> {
        // parse inbox
        let file = PathBuf::from(INPUT_FILENAME);
        let mut inbox = parse_failures(file)?;
        inbox.sort_by( |a, b| a.partial_cmp(b).unwrap() );

        // get confirmation file data
        for f in get_last_n_files(self.files_to_parse)? {
            for cnf_row in parse_file(f.path())? {
                inbox
                    .iter_mut()
                    .filter(|f| **f == cnf_row)
                    .for_each(|f| f.set_confirmation_row_data(cnf_row.clone()));
            }

            let has_confirmation_row = inbox
                .iter()
                .filter(|f| !f.has_confirmation_row())
                .count();

            if has_confirmation_row == 0 {
                break;
            }
        }

        // get orders from cohv
        let userprofile = match std::env::var_os("USERPROFILE") {
            Some(path) => path,
            None => panic!("Could not locate env variable `USERPROFILE`")
        };
    
        let path = PathBuf::from(format!("{}/Documents/SAP/SAP GUI/export.xlsx", userprofile.to_str().unwrap()));

        if !path.exists() {
            self.status = format!("Could not locate export file {}", path.display());
            return Ok(());
        }
    
        for order in parse_cohv_xl(PathBuf::from(path))? {
            match order {
                Order::PlannedOrder(mut data) => {
                    'inbox: for failure in &mut inbox {
                        if failure.mark == data.mark {
                            let order = failure.apply_order_unchecked(data);
    
                            match order {
                                Some(d) => data = d,

                                // break loop if order is 100% applied
                                None => break 'inbox
                            }
                        }
                    }
                },
                _ => ()
            }
        }

        for f in &inbox {
            match f.status() {
                FailureMatchStatus::NoConfirmationRow => {
                    eprintln!("{}\t<{}, {}> has no confirmation row", f.mark, f.wbs, f.program);
                },
                FailureMatchStatus::NotEnoughOrdersApplied(qty) => {
                    eprintln!("{}\t<{}, {}> missing orders for qty of {}/{}", f.mark, f.wbs, f.program, qty, f.qty);
                },
                _ => ()
            }
        }

        let new_inbox: Vec<String> = inbox.iter()
            .map(|f| f.new_inbox_text())
            .filter(Option::is_some)
            .map(Option::unwrap)
            .collect();

        if new_inbox.len() > 0 {
            fs::write("new_inbox.txt", new_inbox.join("\n"))?;
        }

        let prodfile = paths::timestamped_file("Production", "ready");
        let mut records: Vec<CnfFileRow> = Vec::new();
        inbox.iter_mut()
            .map(|f| f.generate_output())
            .for_each(|r| {
                match r {
                    Ok(results) => records.extend(results),
                    Err(e) => eprintln!("{}", e),
                }
            });

        write_file(records, prodfile.into())?;
        if self.auto_move_files {
            self.move_prodfiles()?;
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
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string("auto_move", self.auto_move_files.to_string())
    }

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
                        self.status = match self.generate_parts() {
                            Ok(_) => "parts list generated".into(),
                            Err(e) => format!("Error generating partslist: {}", e)
                        };
                    }
                    if ui.button("Genereate confirmation file").clicked() {
                        self.status = "generating confirmation file...".into();

                        // TODO: move this to another thread because it takes a while
                        self.status = match self.generate_comparison() {
                            Ok(_) => "confirmation file generated".into(),
                            Err(e) => format!("Error generating confirmation file: {}", e)
                        };
                    }
                    if ui.button("Move confirmation file(s)").clicked() {
                        self.status = match self.move_prodfiles() {
                            Ok(_) => "file(s) moved".into(),
                            Err(e) => format!("Error moving files: {}", e)
                        };
                    }

                    // TODO: progress bar
                    // let progress = 0f64;
                    // let progress_bar = egui::ProgressBar::new(progress)
                    //     .show_percentage()
                    //     .animate(*animate_progress_bar);
                    // *animate_progress_bar = ui
                    //     .add(progress_bar)
                    //     .on_hover_text("The progress bar can be animated!")
                    //     .hovered();

                    ui.collapsing("Options", |ui| {

                        ui.checkbox(&mut self.auto_move_files, "Automatically move files after generation");
                        
                        ui.horizontal_centered(|ui| {
                            ui.label("Files to search");
                            ui.add(
                                egui::DragValue::new(&mut self.files_to_parse)
                                    .speed(10.0)
                                    .clamp_range(10..=self.max_files)
                                    .custom_formatter(|n, _| {
                                        if n == self.max_files as f64 {
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

#[cfg(test)]
mod tests {
    use super::SapInboxApp;

    #[test]
    fn comparison() {
        let mut app = SapInboxApp {
            files_to_parse: 1000,

            ..Default::default()
        };

        if let Err(e) = app.generate_comparison() {
            eprintln!("{}", e);
        }
    }
}
