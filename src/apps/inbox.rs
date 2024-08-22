use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::{fs, io};

use eframe::{self, egui};

use crate::api::{CnfFileRow, IssueFileRow, Order};
use crate::inbox::cnf_files::{self, get_last_n_files, parse_file, write_file};
use crate::inbox::parsers::{parse_cohv_xl, parse_failures};
use crate::inbox::{Failure, FailureMatchStatus};
use crate::paths::{self, timestamped_file};

const MAX_FILES: usize = 2000;

#[derive(Debug)]
enum BackgroundTask {
    IssueAll {
        errors: Vec<String>,
        move_files: bool,
    },
}

#[derive(Debug, Default)]
struct Log {
    lines: Mutex<Vec<String>>,
}

impl Log {
    pub fn push(&self, val: &dyn ToString) {
        let mut log = self.lines.lock().unwrap();

        log.push(val.to_string());
    }
}

impl log::Log for Log {
    fn flush(&self) {}

    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() < log::Level::Debug
    }

    fn log(&self, record: &log::Record) {
        self.push(&record.args().to_string());
    }
}

impl ToString for Log {
    fn to_string(&self) -> String {
        let log = self.lines.lock().unwrap();

        log.join("\n")
    }
}

#[derive(Debug)]
pub struct SapInboxApp {
    files_to_parse: usize,
    max_files: usize,
    auto_move_files: bool,

    inbox_errors: String,
    parts_list: String,
    new_inbox: String,
    log: Arc<Log>,

    popup_error: String,
    background_agent: mpsc::Sender<BackgroundTask>,
}

impl Default for SapInboxApp {
    fn default() -> Self {
        Self {
            files_to_parse: 200,
            max_files: cnf_files::get_num_files().unwrap_or(MAX_FILES),
            auto_move_files: true,
            inbox_errors: String::new(),
            new_inbox: String::new(),
            background_agent: mpsc::channel().0,
            parts_list: Default::default(),
            log: Default::default(),
            popup_error: Default::default(),
        }
    }
}

impl SapInboxApp {
    const NAME: &'static str = "SAP Inbox Errors";

    pub async fn new() -> eframe::AppCreator {
        let log = Arc::new(Log::default());
        let app_log = Arc::clone(&log);

        let (tx, rx) = mpsc::channel();
        tokio::spawn(async move {
            while let Ok(task) = rx.recv() {
                match task {
                    BackgroundTask::IssueAll { errors, move_files } => {
                        log::info!("Issue all errors requested");

                        if let Err(e) = Self::issue_all(errors).await {
                            log::error!("{:?}", e);
                        }

                        if move_files {
                            for entry in glob::glob("Issue_*.ready").unwrap() {
                                if let Ok(file) = entry {
                                    let mut to = paths::SAP_OUTBOUND.to_path_buf();
                                    to.push(&file);

                                    fs::copy(&file, to).unwrap();
                                    fs::remove_file(&file).unwrap();

                                    log::info!("Moved file {}", &file.display());
                                }
                            }
                        }
                    }
                }
            }
        });

        Box::new(|cc| Box::new(Self::init(cc, tx, app_log)))
    }

    pub async fn run() -> eframe::Result<()> {
        eframe::run_native(Self::NAME, Self::win_opts(), Self::new().await)
    }

    fn win_opts() -> eframe::NativeOptions {
        eframe::NativeOptions {
            centered: true,
            default_theme: eframe::Theme::Light,

            ..Default::default()
        }
    }

    fn init(
        cc: &eframe::CreationContext<'_>,
        background_agent: mpsc::Sender<BackgroundTask>,
        log: Arc<Log>,
    ) -> Self {
        let (auto_move_files, inbox_errors, new_inbox) = match cc.storage {
            Some(storage) => (
                storage.get_string("auto_move").unwrap_or_default() == "true",
                storage.get_string("inbox").unwrap_or_default(),
                storage.get_string("new_inbox").unwrap_or_default(),
            ),
            None => (false, "".into(), "".into()),
        };

        Self {
            files_to_parse: 200,
            max_files: cnf_files::get_num_files().unwrap_or(MAX_FILES),
            auto_move_files,
            log,
            inbox_errors,
            new_inbox,
            background_agent,

            ..Default::default()
        }
    }

    fn inbox_errors(&self) -> std::str::Split<&str> {
        self.inbox_errors
            .trim_end()
            // .trim_end_matches("\n")
            .split("\n")
    }

    fn log(&mut self, val: impl AsRef<str>) {
        let log = self.log.clone();
        log.push(&val.as_ref().to_string());
    }

    pub fn generate_parts(&mut self) -> anyhow::Result<()> {
        if self.inbox_errors.is_empty() {
            return Err(anyhow!("No inbox errors to parse"));
        }

        let inbox = parse_failures(self.inbox_errors());

        // get marks only from failures
        let (parsed, errors): (Vec<_>, Vec<_>) = inbox.into_iter().partition(Result::is_ok);

        // log errors
        errors
            .into_iter()
            .for_each(|e| self.log(e.unwrap_err().to_string()));

        let mut marks: Vec<String> = parsed.into_iter().map(|f| f.unwrap().mark).collect();

        // remove duplicates
        marks.sort();
        marks.dedup();

        // cannot delimit on '\n' because applications like SAP and Excel don't read this as multiple lines
        self.parts_list = marks.join("\r\n");

        Ok(())
    }

    async fn issue_all(errors: Vec<String>) -> anyhow::Result<()> {
        // parse inbox
        let mut inbox: Vec<Failure> = parse_failures(errors.into_iter())
            .into_iter()
            .filter(|f| f.is_ok())
            .map(Result::unwrap)
            .collect();
        inbox.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // get issue data
        let mut conn = crate::db::connect_to_sn().await?;
        let issuefile = paths::timestamped_file("Issue", "ready");
        let mut records: Vec<IssueFileRow> = Vec::new();

        for mut inbox_error in inbox {
            let issue_line = inbox_error.generate_issue_output(&mut conn).await;

            match issue_line {
                Ok(result) => records.push(result),
                Err(e) => eprintln!("{}", e),
            };
        }

        write_file(records, issuefile.into())?;

        Ok(())
    }

    pub fn generate_comparison(&mut self) -> anyhow::Result<()> {
        if self.inbox_errors.is_empty() {
            return Err(anyhow!("No inbox errors to parse"));
        }

        // parse inbox
        let mut inbox: Vec<Failure> = parse_failures(self.inbox_errors())
            .into_iter()
            .filter(|f| f.is_ok())
            .map(Result::unwrap)
            .collect();
        inbox.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // get confirmation file data
        for f in get_last_n_files(self.files_to_parse)? {
            for cnf_row in parse_file(f.path())? {
                inbox
                    .iter_mut()
                    .filter(|f| **f == cnf_row)
                    .for_each(|f| f.set_confirmation_row_data(cnf_row.clone()));
            }

            let has_confirmation_row = inbox.iter().filter(|f| !f.has_confirmation_row()).count();

            if has_confirmation_row == 0 {
                break;
            }
        }

        // get orders from cohv
        let userprofile = match std::env::var_os("USERPROFILE") {
            Some(path) => path,
            None => {
                return Err(anyhow!(
                    "Could not locate environment variable `USERPROFILE`"
                ))
            }
        };

        let path = PathBuf::from(format!(
            "{}/Documents/SAP/SAP GUI/export.xlsx",
            userprofile.to_str().unwrap()
        ));

        if !path.exists() {
            return Err(anyhow!("Could not locate export file: {}", path.display()));
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
                                None => break 'inbox,
                            }
                        }
                    }
                }
                _ => (),
            }
        }

        for f in &inbox {
            match f.status() {
                FailureMatchStatus::NoConfirmationRow => {
                    self.log(format!(
                        "{}\t<{}, {}> has no confirmation row",
                        f.mark, f.wbs, f.program
                    ));
                }
                FailureMatchStatus::NotEnoughOrdersApplied(qty) => {
                    self.log(format!(
                        "{}\t<{}, {}> missing orders for qty of {}/{}",
                        f.mark, f.wbs, f.program, qty, f.qty
                    ));
                }
                _ => (),
            }
        }

        let new_inbox: Vec<String> = inbox
            .iter()
            .map(|f| f.new_inbox_text())
            .filter(Option::is_some)
            .map(Option::unwrap)
            .collect();

        self.new_inbox = new_inbox.join("\n");
        if new_inbox.len() > 0 {
            fs::write(timestamped_file("new_inbox", "txt"), new_inbox.join("\n"))?;
        }

        let prodfile = paths::timestamped_file("Production", "ready");
        let mut records: Vec<CnfFileRow> = Vec::new();
        inbox
            .iter_mut()
            .map(|f| f.generate_output())
            .for_each(|r| match r {
                Ok(results) => records.extend(results),
                Err(e) => eprintln!("{}", e),
            });

        write_file(records, prodfile.into())?;
        if self.auto_move_files {
            self.move_prodfiles()?;
        }

        Ok(())
    }

    fn move_prodfiles(&mut self) -> io::Result<()> {
        self.move_files("Production_*.ready")
    }

    fn move_files(&mut self, pattern: &str) -> io::Result<()> {
        for entry in glob::glob(pattern).unwrap() {
            if let Ok(file) = entry {
                let mut to = paths::SAP_OUTBOUND.to_path_buf();
                to.push(&file);

                fs::copy(&file, to)?;
                fs::remove_file(&file)?;

                self.log(format!("Moved file {}", &file.display()))
            }
        }

        Ok(())
    }
}

impl eframe::App for SapInboxApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string("auto_move", self.auto_move_files.to_string());
        storage.set_string("inbox", self.inbox_errors.to_string());
        storage.set_string("new_inbox", self.new_inbox.to_string());
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("action-area").show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                let res_parts = ui.button("Generate parts list");
                let err_parts = ui.make_persistent_id("gen-parts-error-popup");
                egui::popup_below_widget(ui, err_parts, &res_parts, |ui| {
                    ui.style_mut().wrap = Some(false);
                    ui.label(&self.popup_error);
                });

                if res_parts.clicked() {
                    self.log("Generating parts list...");
                    if let Err(e) = self.generate_parts() {
                        self.popup_error = e.to_string();
                        ui.memory_mut(|mem| mem.open_popup(err_parts));
                    }
                }

                let res_cnf = ui.button("Generate confirmation file");
                let err_cnf = ui.make_persistent_id("gen-cnf-error-popup");
                egui::popup_below_widget(ui, err_cnf, &res_cnf, |ui| {
                    ui.style_mut().wrap = Some(false);
                    ui.label(&self.popup_error);
                });
                if res_cnf.clicked() {
                    self.log("Generating confirmation file...");

                    // TODO: move this to another thread because it takes a while
                    match self.generate_comparison() {
                        Ok(_) => self.log("Confirmation file generated"),
                        Err(e) => {
                            self.popup_error = e.to_string();
                            ui.memory_mut(|mem| mem.open_popup(err_cnf));
                        }
                    }
                }

                if ui.button("Move confirmation file(s)").clicked() {
                    match self.move_prodfiles() {
                        Ok(_) => self.log("File(s) moved"),
                        Err(e) => self.log(e.to_string()),
                    }
                }
            });
        });

        egui::TopBottomPanel::top("inbox-errors")
            .resizable(true)
            .min_height(100.)
            .show(ctx, |ui| {
                ui.heading("Inbox Errors");
                egui::ScrollArea::both()
                    .id_source("inbox scroll area")
                    .max_height(200.)
                    .show(ui, |ui| {
                        ui.style_mut().wrap = Some(false);
                        egui::TextEdit::multiline(&mut self.inbox_errors)
                            .desired_width(f32::INFINITY)
                            .show(ui);
                    });

                ui.horizontal(|ui| {
                    if ui.button("Clear inbox errors").clicked() {
                        self.inbox_errors.clear();
                    }

                    if !self.inbox_errors.is_empty() {
                        let res_issue = ui.button("Issue all errors");

                        if res_issue.clicked() {
                            self.background_agent
                                .send(BackgroundTask::IssueAll {
                                    errors: self.inbox_errors().map(ToString::to_string).collect(),
                                    move_files: self.auto_move_files,
                                })
                                .unwrap();
                        }
                    }
                });
            });

        egui::TopBottomPanel::bottom("options").show(ctx, |ui| {
            ui.collapsing("Options", |ui| {
                ui.checkbox(
                    &mut self.auto_move_files,
                    "Automatically move files after generation",
                );

                ui.horizontal(|ui| {
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
                            }),
                    );
                });
            });
        });
        egui::TopBottomPanel::bottom("log")
            .resizable(true)
            .min_height(100.)
            .show(ctx, |ui| {
                ui.heading("Log");
                egui::ScrollArea::both()
                    .stick_to_bottom(true)
                    .id_source("log scroll area")
                    .max_height(100.)
                    .show(ui, |ui| {
                        let log = Arc::clone(&self.log);
                        ui.label(log.to_string())
                        // egui::TextEdit::multiline(&mut self.log)
                        //     .desired_width(f32::INFINITY)
                        //     .show(ui);
                    });
            });

        egui::SidePanel::left("parts")
            // .resizable(true)
            .min_width(150.)
            .show(ctx, |ui| {
                ui.heading("Parts List");
                egui::ScrollArea::vertical()
                    .id_source("parts scroll area")
                    // .max_height(100.)
                    .show_rows(
                        ui,
                        ui.text_style_height(&egui::TextStyle::Body),
                        10,
                        |ui, rng| {
                            let display = self
                                .parts_list
                                .split('\n')
                                .into_iter()
                                .skip(rng.start)
                                .take(rng.end - rng.start)
                                .collect::<Vec<_>>()
                                .join("\n");

                            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);

                            if ui
                                .add(egui::Label::new(display).sense(egui::Sense::click()))
                                .on_hover_ui(|ui| {
                                    ui.label("Click to copy");
                                })
                                .clicked()
                            {
                                ui.output_mut(|out| {
                                    out.copied_text = String::from(&self.parts_list)
                                });
                                self.log("Parts list copied to clipboard.")
                            }
                        },
                    );
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                ui.heading("Not Matched");
                egui::ScrollArea::both()
                    .id_source("not-matched scroll area")
                    .max_height(100.)
                    .show(ui, |ui| {
                        egui::TextEdit::multiline(&mut self.new_inbox)
                            .desired_width(f32::INFINITY)
                            .show(ui);
                    });

                if ui.button("Clear not matched").clicked() {
                    self.new_inbox.clear();
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
