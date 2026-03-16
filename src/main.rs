#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{self, DroppedFile, Frame, RichText};
use lib::chor;
use lib::flip;
use lib::pathplanner;
use std::ffi::OsStr;
use std::{
    fs::File,
    io::{Result},
    path::{Path, PathBuf},
};
use walkdir::{DirEntry, WalkDir};

use crate::lib::{
    plot::{self, Plotter},
};

mod lib {
    pub mod chor;
    pub mod flip;
    pub mod pathplanner;
    pub mod plot;
    pub mod util;
}

fn main() -> eframe::Result {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_active(true)
            .with_maximized(true)
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "Choreo Path Flipper",
        opts,
        Box::new(|_| Ok(Box::new(PathFlip::new()))),
    )
}

#[derive(Clone, Copy, PartialEq)]
enum FlipFileType {
    Choreo,
    Pathplanner,
    PathplannerAuto { is_chor: bool },
}

impl FlipFileType {
    pub fn get_ext(&self) -> String {
        return match self {
            Self::Choreo => String::from("traj"),
            Self::Pathplanner => String::from("path"),
            Self::PathplannerAuto { is_chor: _ } => String::from("auto"),
        };
    }

    pub fn check_file(&self, f: &File) -> bool {
        match self {
            Self::Choreo => {
                let parsed: serde_json::Result<chor::ChoreoData> = serde_json::from_reader(f);
                match parsed {
                    Ok(_) => true,
                    Err(err) => {
                        println!("{}", err);
                        false
                    }
                }
            }
            Self::Pathplanner => {
                let parsed: serde_json::Result<pathplanner::path::PathData> =
                    serde_json::from_reader(f);
                match parsed {
                    Ok(_) => true,
                    Err(err) => {
                        println!("{}", err);
                        false
                    }
                }
            }
            Self::PathplannerAuto { is_chor: _ } => {
                let parsed: serde_json::Result<pathplanner::auto::AutoData> =
                    serde_json::from_reader(f);
                match parsed {
                    Ok(_) => true,
                    Err(err) => {
                        println!("{}", err);
                        false
                    }
                }
            }
        }
    }
}

struct PathFlip {
    dropped_files: Vec<egui::DroppedFile>,
    auto_files: Vec<PathBuf>,
    auto_file_names: Vec<String>,
    auto_file_prefs: Vec<String>,
    auto_file_valids: Vec<bool>,
    picked_path: Option<String>,
    flip_same_alliance: bool,
    outputname: String,
    recalc_path: bool,
    outputname_valid: bool,
    path_is_valid_file: bool,
    path_type: FlipFileType,
    plotter: plot::DualPlotter,
    robot_x_m: f64,
    robot_y_m: f64,
    robot_x_m_exp: String,
    robot_y_m_exp: String,
    units_is_imp: bool,
    modal_open: bool,
    use_curr_dir: bool,
    write_status: String,
    write_err: bool,
    dir_prefx: String,
    chassis_color: [u8; 3],
}

impl Default for PathFlip {
    fn default() -> Self {
        Self {
            dropped_files: Default::default(),
            auto_files: Vec::new(),
            auto_file_names: Vec::new(),
            auto_file_prefs: Vec::new(),
            auto_file_valids: Vec::new(),
            picked_path: Default::default(),
            flip_same_alliance: true,
            outputname: Default::default(),
            path_type: FlipFileType::Choreo,
            recalc_path: false,
            path_is_valid_file: false,
            plotter: Default::default(),
            robot_x_m: 0.889,
            robot_y_m: 0.889,
            robot_x_m_exp: "0.889".to_string(),
            robot_y_m_exp: "0.889".to_string(),
            modal_open: false,
            units_is_imp: false,
            outputname_valid: false,
            use_curr_dir: true,
            dir_prefx: "C:\\".to_string(),
            write_status: Default::default(),
            write_err: false,
            chassis_color: egui::Color32::PURPLE
                .to_array()
                .split_last()
                .unwrap()
                .1
                .try_into()
                .unwrap(),
        }
    }
}

impl PathFlip {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn load_file(&mut self, path: &PathBuf) {
        let path_str = path.display().to_string();
        self.picked_path = Some(path_str);
        self.path_is_valid_file = false;
        if let Some(picked_path) = &self.picked_path {
            let ext = path.extension().unwrap_or(OsStr::new(""));
            if ext
                == (FlipFileType::PathplannerAuto { is_chor: false })
                    .get_ext()
                    .as_str()
            {
                self.picked_path = Some(path.display().to_string());
                self.path_is_valid_file = false;
                if let Some(parent) = path.parent() {
                    if let Ok(file) = File::open(&path) {
                        if let Ok(data) =
                            serde_json::from_reader::<&File, pathplanner::auto::AutoData>(&file)
                        {
                            let names = data.get_filenames();
                            let path = if names.1 {
                                parent
                                    .parent()
                                    .unwrap() // autos -> pathplanner
                                    .parent()
                                    .unwrap() // pathplanner -> deploy
                                    .join("choreo")
                            } else {
                                parent
                                    .parent()
                                    .unwrap() // autos -> pathplanner
                                    .join("paths")
                            };
                            self.auto_files.clear();
                            self.auto_files
                                .extend(names.0.iter().map(|s| PathBuf::from(path.join(s))));
                            self.auto_file_prefs =
                                self.auto_files.iter().map(|_| String::new()).collect();
                            self.auto_file_names =
                                self.auto_files.iter().map(|_| String::new()).collect();
                            self.auto_file_valids = self.auto_files.iter().map(|_| false).collect();
                            self.path_type = FlipFileType::PathplannerAuto { is_chor: names.1 };
                            self.path_is_valid_file = true;
                            self.recalc_path = true;
                        }
                    }
                }
            } else {
                if let Ok(file) = File::open(picked_path) {
                    for i in [FlipFileType::Choreo, FlipFileType::Pathplanner] {
                        if i.get_ext().as_str() == ext && i.check_file(&file) {
                            self.path_is_valid_file = true;
                            self.recalc_path = true;
                            self.path_type = i;
                            self.auto_files.clear();
                        }
                    }
                }
            }
        }
    }
}

impl eframe::App for PathFlip {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let col = egui::Color32::from_rgb(
            self.chassis_color[0],
            self.chassis_color[1],
            self.chassis_color[2],
        );
        if self.modal_open {
            egui::Window::new("Config Options")
                .collapsible(false)
                .resizable(true)
                .movable(false)
                .open(&mut self.modal_open)
                .show(ctx, |ui| {
                    let mut x_color = egui::Color32::RED;
                    let mut y_color = egui::Color32::RED;
                    if let Ok(x) = self.robot_x_m_exp.parse::<f64>() {
                        self.robot_x_m = x * (if self.units_is_imp { 0.0254 } else { 1.0 });
                        x_color = egui::Color32::GREEN;
                    }
                    if let Ok(y) = self.robot_y_m_exp.parse::<f64>() {
                        self.robot_y_m = y * (if self.units_is_imp { 0.0254 } else { 1.0 });
                        y_color = egui::Color32::GREEN;
                    }
                    ui.checkbox(&mut self.units_is_imp, "Use Imperial Units (in)");
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Robot Width (X): ")
                                .monospace()
                                .color(x_color),
                        );
                        ui.text_edit_singleline(&mut self.robot_x_m_exp);
                        ui.label(format!(
                            "{} ({} {})",
                            if self.units_is_imp { "in" } else { "m" },
                            if self.units_is_imp {
                                self.robot_x_m
                            } else {
                                self.robot_x_m / 0.0254
                            },
                            if self.units_is_imp { "m" } else { "in" }
                        ));
                    });
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Robot Height (Y): ")
                                .monospace()
                                .color(y_color),
                        );
                        ui.text_edit_singleline(&mut self.robot_y_m_exp);
                        ui.label(format!(
                            "{} ({} {})",
                            if self.units_is_imp { "in" } else { "m" },
                            if self.units_is_imp {
                                self.robot_y_m
                            } else {
                                self.robot_y_m / 0.0254
                            },
                            if self.units_is_imp { "m" } else { "in" }
                        ));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Chassis Color");
                        ui.color_edit_button_srgb(&mut self.chassis_color);
                    });
                    let side_length_x = self.robot_x_m * 100.0;
                    let side_length_y = self.robot_y_m * 100.0;

                    let rect_size = egui::Vec2::new(side_length_x as f32, side_length_y as f32);
                    Frame::new()
                        .stroke(egui::Stroke::new(4.0, col))
                        .show(ui, |ui| {
                            ui.allocate_space(rect_size);
                        });
                });
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Config").clicked() {
                self.modal_open = true;
            }

            ui.label("Drag-and-drop or select file").highlight();

            if !self.dropped_files.is_empty() {
                ui.group(|ui| {
                    ui.label("Dropped files:");

                    for file in self.dropped_files.clone() {
                        let mut info = if let Some(path) = &file.path {
                            path.display().to_string()
                        } else if !file.name.is_empty() {
                            file.name.clone()
                        } else {
                            "???".to_owned()
                        };

                        let mut additional_info = vec![];

                        if !file.mime.is_empty() {
                            additional_info.push(format!("type: {}", file.mime));
                        }
                        if let Some(bytes) = &file.bytes {
                            additional_info.push(format!("{} bytes", bytes.len()));
                        }
                        if !additional_info.is_empty() {
                            info += &format!(" ({})", additional_info.join(", "));
                        }

                        if ui.button(info).clicked() {
                            if let Some(path) = &file.path {
                                self.load_file(path);
                            }
                        }
                    }

                    if ui.button("Clear").clicked() {
                        self.dropped_files.clear();
                    }
                });
            }

            ui.horizontal(|ui| {
                if ui.button("Open file…").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        self.load_file(&path);
                    }
                }

                if ui.button("Open folder...").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.dropped_files
                            .extend(collect_files(&path, &["auto", "path", "traj"]));
                    }
                }

                if self.picked_path.is_some() {
                    if ui.button("Close File").clicked() {
                        self.picked_path = None;
                        self.path_is_valid_file = false;
                        self.auto_files.clear();
                        self.outputname.clear();
                        self.outputname_valid = false;
                        self.write_status.clear();
                        ctx.style_mut(|f| {
                            f.visuals.override_text_color = None;
                        });
                    }
                }
            });

            if let Some(picked_path) = &self.picked_path {
                ui.horizontal(|ui| {
                    ui.label("Selected file:");
                    ui.label(RichText::new(picked_path).monospace().color(
                        if self.path_is_valid_file {
                            egui::Color32::GREEN
                        } else {
                            egui::Color32::RED
                        },
                    ));
                });
            }

            if ui
                .checkbox(
                    &mut self.flip_same_alliance,
                    "Flip across the Y axis (same alliance right/left)",
                )
                .changed()
            {
                self.recalc_path = true;
            }
            ui.checkbox(
                &mut self.use_curr_dir,
                "Use Selected Path Directory for output",
            );
            if let Some(picked_p) = &self.picked_path {
                let outputnamelabel = ui.label(format!(
                    "Output file name -- {}",
                    PathBuf::from(picked_p)
                        .file_name()
                        .unwrap()
                        .display()
                        .to_string()
                ));
                if self.use_curr_dir {
                    self.dir_prefx = PathBuf::from(picked_p)
                        .parent()
                        .unwrap()
                        .display()
                        .to_string()
                        + "\\";
                } else {
                    self.dir_prefx = "C:\\".to_string();
                }
                ui.horizontal(|ui| {
                    ui.label(&self.dir_prefx);
                    ui.add_enabled(
                        self.path_is_valid_file && self.picked_path.is_some(),
                        egui::TextEdit::singleline(&mut self.outputname).background_color(
                            if self.outputname_valid {
                                egui::Visuals::dark().extreme_bg_color
                            } else {
                                egui::Color32::DARK_RED
                            },
                        ),
                    )
                    .labelled_by(outputnamelabel.id);
                    ui.label(".".to_string() + &self.path_type.get_ext());
                });
                if let FlipFileType::PathplannerAuto {
                    is_chor: is_chorchor,
                } = self.path_type
                {
                    for i in 0..self.auto_files.len() {
                        let sublabelname = ui.label(format!(
                            "Path {} -- {}",
                            i + 1,
                            self.auto_files[i]
                                .file_name()
                                .unwrap()
                                .display()
                                .to_string()
                        ));
                        if self.use_curr_dir {
                            self.auto_file_prefs[i] =
                                self.auto_files[i].parent().unwrap().display().to_string() + "\\";
                        } else {
                            self.auto_file_prefs[i] = "C:\\".to_string();
                        }
                        self.auto_file_valids[i] = outputfile_valid_list(
                            &self.auto_file_names[i],
                            &self.auto_files[i].display().to_string(),
                            &self.auto_file_names[0..i],
                        );
                        ui.horizontal(|ui| {
                            ui.label(&self.auto_file_prefs[i]);
                            ui.add_enabled(
                                self.path_is_valid_file && self.picked_path.is_some(),
                                egui::TextEdit::singleline(&mut self.auto_file_names[i])
                                    .background_color(if self.auto_file_valids[i] {
                                        egui::Visuals::dark().extreme_bg_color
                                    } else {
                                        egui::Color32::DARK_RED
                                    }),
                            )
                            .labelled_by(sublabelname.id);
                            ui.label(".".to_string() + if is_chorchor { "traj" } else { "path" });
                        });
                    }
                }
            }
            if !self.write_status.is_empty() {
                ui.label(
                    RichText::new(&self.write_status)
                        .monospace()
                        .color(if self.write_err {
                            egui::Color32::RED
                        } else {
                            egui::Color32::GREEN
                        }),
                );
            }
            self.outputname_valid = outputfile_valid(
                &self.outputname,
                self.picked_path.as_ref().unwrap_or(&String::new()),
            );
            if let Some(path) = &self.picked_path {
                if self.path_is_valid_file
                    && self.outputname_valid
                    && !path.is_empty()
                    && (!matches!(
                        self.path_type,
                        FlipFileType::PathplannerAuto { is_chor: false }
                            | FlipFileType::PathplannerAuto { is_chor: true }
                    ) || self.auto_file_valids.iter().all(|b| *b))
                {
                    if ui.button("Flip").clicked() {
                        let mut outputfile = PathBuf::from(&self.dir_prefx);
                        outputfile.push(&self.outputname);
                        outputfile.set_extension(self.path_type.get_ext());
                        let stat = self.plotter.send_flip(
                            self.picked_path.as_ref().unwrap().to_owned(),
                            outputfile.display().to_string(),
                            self.flip_same_alliance,
                            Some(&self.auto_file_names),
                        );
                        self.write_status = format!("{:?}", stat);
                        self.write_err = stat.is_err();
                    }
                }
            }
            if let Some(picked_pth) = &mut self.picked_path {
                if self.path_is_valid_file {
                    if self.recalc_path {
                        self.plotter.reset();
                        self.plotter
                            .set_plot_type(&self.path_type, self.auto_files.clone());
                        self.plotter
                            .gen(
                                picked_pth,
                                self.robot_y_m,
                                self.robot_x_m,
                                self.flip_same_alliance,
                            )
                            .unwrap();
                        self.recalc_path = false;
                    }
                    self.plotter.plot(&col, ctx, ui).unwrap();
                }
            }
        });

        drop_file_preview(ctx);

        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                self.dropped_files.extend(i.raw.dropped_files.clone());
            }
        });
    }
}

fn outputfile_valid(name: &String, inputname: &String) -> bool {
    !name.is_empty()
        && name != inputname
        && !name
            .chars()
            .any(|c| matches!(c, '.' | '<' | '>' | ':' | '"' | '/' | '|' | '?' | '*'))
        && !matches!(
            name.as_str(),
            "CON"
                | "PRN"
                | "AUX"
                | "NUL"
                | "COM1"
                | "COM2"
                | "COM3"
                | "COM4"
                | "COM5"
                | "COM6"
                | "COM7"
                | "COM8"
                | "COM9"
                | "LPT1"
                | "LPT2"
                | "LPT3"
                | "LPT4"
                | "LPT5"
                | "LPT6"
                | "LPT7"
                | "LPT8"
                | "LPT9"
        )
}

fn outputfile_valid_list(name: &String, inputname: &String, last_names: &[String]) -> bool {
    !name.is_empty()
        && inputname != name
        && !last_names.contains(name)
        && !name
            .chars()
            .any(|c| matches!(c, '.' | '<' | '>' | ':' | '"' | '/' | '|' | '?' | '*'))
        && !matches!(
            name.as_str(),
            "CON"
                | "PRN"
                | "AUX"
                | "NUL"
                | "COM1"
                | "COM2"
                | "COM3"
                | "COM4"
                | "COM5"
                | "COM6"
                | "COM7"
                | "COM8"
                | "COM9"
                | "LPT1"
                | "LPT2"
                | "LPT3"
                | "LPT4"
                | "LPT5"
                | "LPT6"
                | "LPT7"
                | "LPT8"
                | "LPT9"
        )
}

fn collect_files(folder: &Path, extensions: &[&str]) -> Vec<DroppedFile> {
    let paths = WalkDir::new(folder)
        .into_iter()
        .filter_map(|d| Result::ok(Ok(d.ok()?)))
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            if let Some(ext) = entry.path().extension() {
                let ext = ext.to_string_lossy().to_lowercase();
                extensions.iter().any(|e| e == &ext)
            } else {
                false
            }
        })
        .map(|entry: DirEntry| entry.path().to_path_buf())
        .collect::<Vec<std::path::PathBuf>>();
    paths
        .into_iter()
        .map(|path| DroppedFile {
            path: Some(path),
            name: String::new(),
            mime: String::new(),
            last_modified: Option::None,
            bytes: None,
        })
        .collect()
}

fn drop_file_preview(ctx: &egui::Context) {
    use egui::{Align2, Color32, Id, LayerId, Order, TextStyle};
    use std::fmt::Write as _;

    if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
        let text = ctx.input(|i| {
            let mut text = "Dropping files:\n".to_owned();
            for file in &i.raw.hovered_files {
                if let Some(path) = &file.path {
                    write!(text, "\n{}", path.display()).ok();
                } else if !file.mime.is_empty() {
                    write!(text, "\n{}", file.mime).ok();
                } else {
                    text += "\n???";
                }
            }
            text
        });

        let painter =
            ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

        let screen_rect = ctx.content_rect();
        painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
        painter.text(
            screen_rect.center(),
            Align2::CENTER_CENTER,
            text,
            TextStyle::Heading.resolve(&ctx.style()),
            Color32::WHITE,
        );
    }
}
