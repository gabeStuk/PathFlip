#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{self, Color32, ColorImage, Frame, RichText};
use egui_plot::{Line, LineStyle, PlotImage, PlotItem, PlotPoint};
use lib::chor::chor;
use std::{
    f32::consts::FRAC_PI_2,
    fs::File,
    io::{Result, Write},
    path::{Path, PathBuf},
};

mod lib {
    pub mod chor;
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
        Box::new(|cc| Ok(Box::new(CFlip::new(cc)))),
    )
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CFlip {
    dropped_files: Vec<egui::DroppedFile>,
    picked_path: Option<String>,
    flip_same_alliance: bool,
    outputname: String,
    outputname_valid: bool,
    path_is_chor_traj: bool,
    robot_x_m: f64,
    robot_y_m: f64,
    store_state: bool,
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

impl Default for CFlip {
    fn default() -> Self {
        Self {
            dropped_files: Default::default(),
            picked_path: Default::default(),
            flip_same_alliance: true,
            outputname: Default::default(),
            path_is_chor_traj: false,
            robot_x_m: 0.889,
            robot_y_m: 0.889,
            robot_x_m_exp: "0.889".to_string(),
            robot_y_m_exp: "0.889".to_string(),
            modal_open: false,
            units_is_imp: false,
            store_state: true,
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

impl CFlip {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for CFlip {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if !self.store_state {
            *self = CFlip::default();
        }
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

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
                    ui.checkbox(&mut self.store_state, "Save app state on exit");
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

                    for file in &self.dropped_files {
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
                                self.picked_path = Some(path.display().to_string());
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
                        self.picked_path = Some(path.display().to_string());
                        self.path_is_chor_traj = false;
                        if let Some(picked_path) = &self.picked_path {
                            if let Ok(file) = File::open(picked_path) {
                                if let Ok(_) =
                                    serde_json::from_reader::<&File, chor::ChoreoData>(&file)
                                {
                                    self.path_is_chor_traj = true;
                                }
                            }
                        }
                    }
                }

                if self.picked_path.is_some() {
                    if ui.button("Close File").clicked() {
                        self.picked_path = None;
                        self.path_is_chor_traj = false;
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
                        if self.path_is_chor_traj {
                            egui::Color32::GREEN
                        } else {
                            egui::Color32::RED
                        },
                    ));
                });
            }

            ui.checkbox(
                &mut self.flip_same_alliance,
                "Flip across the X axis (same alliance right/left)",
            );
            ui.checkbox(
                &mut self.use_curr_dir,
                "Use Selected Path Directory for output",
            );
            let outputnamelabel = ui.label("Output file name");
            ui.horizontal(|ui| {
                if let Some(picked_p) = &self.picked_path {
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
                    ui.label(&self.dir_prefx);
                }
                ui.add_enabled(
                    self.path_is_chor_traj && self.picked_path.is_some(),
                    egui::TextEdit::singleline(&mut self.outputname).background_color(
                        if self.outputname_valid {
                            egui::Visuals::dark().extreme_bg_color
                        } else {
                            egui::Color32::DARK_RED
                        },
                    ),
                )
                .labelled_by(outputnamelabel.id);
                ui.label(".traj");
            });
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
            self.outputname_valid = !self.outputname.is_empty()
                && !self
                    .outputname
                    .chars()
                    .any(|c| matches!(c, '.' | '<' | '>' | ':' | '"' | '/' | '|' | '?' | '*'))
                && !matches!(
                    self.outputname.as_str(),
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
                );
            if let Some(path) = &self.picked_path {
                if self.path_is_chor_traj && self.outputname_valid && !path.is_empty() {
                    if ui.button("Flip").clicked() {
                        let mut outputfile = PathBuf::from(&self.dir_prefx);
                        outputfile.push(&self.outputname);
                        outputfile.set_extension("traj");
                        if self.flip_same_alliance {
                            let stat = flip_same_alliance(
                                self.picked_path.as_ref().unwrap().to_owned(),
                                outputfile.display().to_string(),
                            );
                            self.write_status = format!("{:?}", stat);
                            self.write_err = stat.is_err();
                        } else {
                            let stat = flip_alliance(
                                self.picked_path.as_ref().unwrap().to_owned(),
                                outputfile.display().to_string(),
                            );
                            self.write_status = format!("{:?}", stat);
                            self.write_err = stat.is_err();
                        }
                    }
                }
            }
            if let Some(picked_pth) = &self.picked_path {
                if self.path_is_chor_traj {
                    choreo_plot(&self, &col, ctx, ui, picked_pth, self.flip_same_alliance).unwrap();
                }
            }
        });

        drop_file_preview(ctx);

        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                self.dropped_files.clone_from(&i.raw.dropped_files);
            }
        });
    }
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

        let screen_rect = ctx.screen_rect();
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

fn color_lerp(low: egui::Color32, high: egui::Color32, t: f64) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    egui::Color32::from_rgb(
        (low.r() as f64 * (1.0 - t) + high.r() as f64 * t) as u8,
        (low.g() as f64 * (1.0 - t) + high.g() as f64 * t) as u8,
        (low.b() as f64 * (1.0 - t) + high.b() as f64 * t) as u8,
    )
}

fn choreo_plot(
    _self: &CFlip,
    col: &egui::Color32,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    filepath: &String,
    same_alli: bool,
) -> Result<()> {
    use egui_plot::{Line, Plot};
    let file = File::open(filepath)?;
    let data: chor::ChoreoData = serde_json::from_reader(&file)?;
    let gray_blend = Color32::from_rgba_unmultiplied(
        Color32::GRAY.r(),
        Color32::GRAY.g(),
        Color32::GRAY.b(),
        50_u8,
    );
    let gray_blend2 = Color32::from_rgba_unmultiplied(
        Color32::GRAY.r(),
        Color32::GRAY.g(),
        Color32::GRAY.b(),
        150_u8,
    );
    let samples = &data.trajectory.samples;
    let waypoints = &data.params.waypoints;
    let min_vel = samples
        .iter()
        .map(|s| (s.vx * s.vx + s.vy * s.vy).sqrt())
        .fold(f64::INFINITY, f64::min);
    let max_vel = samples
        .iter()
        .map(|s| (s.vx * s.vx + s.vy * s.vy).sqrt())
        .fold(f64::NEG_INFINITY, f64::max);
    let range = (max_vel - min_vel).max(0.01);
    let mut sample_segs: Vec<Line> = Vec::new();
    for (i, pair) in samples.windows(2).enumerate() {
        let s0 = &pair[0];
        let s1 = &pair[1];
        let avg_vel =
            ((s0.vx * s0.vx + s0.vy * s0.vy).sqrt() + (s1.vx * s1.vx + s1.vy * s1.vy).sqrt()) / 2.0;
        let t = (avg_vel - min_vel) / range;
        let color = color_lerp(
            egui::Color32::RED,
            egui::Color32::GREEN.blend(gray_blend),
            t,
        );
        let line = Line::new(
            format!("Sample Line {}", i),
            vec![[s0.x as f64, s0.y as f64], [s1.x as f64, s1.y as f64]],
        )
        .color(color)
        .highlight(true);
        sample_segs.push(line);
    }
    let mirr_func = if same_alli {
        chor::flip_xaxis
    } else {
        chor::flip_yaxis
    };
    let mirr_cs = if same_alli {
        chor::ChoreoSample::flip_same_alliance
    } else {
        chor::ChoreoSample::flip_alliance
    };
    let sample_mirr_segs = sample_segs
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let mut smp_window = [
                samples.get(i).unwrap().to_owned(),
                samples.get(i + 1).unwrap().to_owned(),
            ];
            smp_window.iter_mut().for_each(mirr_cs);
            Line::new(
                format!("Mirrored Sample Line {}", i),
                vec![
                    [smp_window[0].x, smp_window[0].y],
                    [smp_window[1].x, smp_window[1].y],
                ],
            )
            .color(s.color().blend(gray_blend).blend(gray_blend))
            .highlight(true)
        })
        .collect::<Vec<_>>();
    let wps = waypoints
        .iter()
        .map(|wp| {
            (
                PlotPoint::from([wp.x.val as f64, wp.y.val as f64]),
                wp.heading.val as f64,
            )
        })
        .collect::<Vec<_>>();
    let mirred_wps = wps
        .iter()
        .map(|wp| (PlotPoint::from(mirr_func(wp.0.x, wp.0.y)), -wp.1))
        .collect::<Vec<_>>();
    let wp_squares = wps
        .iter()
        .enumerate()
        .map(|(i, p)| {
            draw_rotate_square_rect(
                &p.0,
                _self.robot_y_m,
                _self.robot_x_m,
                *col,
                p.1 - FRAC_PI_2 as f64,
                false,
                (i + 1).to_string(),
            )
        })
        .collect::<Vec<_>>();
    let wp_mirr_squares = mirred_wps
        .iter()
        .enumerate()
        .map(|(i, p)| {
            draw_rotate_square_rect(
                &p.0,
                _self.robot_y_m,
                _self.robot_x_m,
                col.blend(gray_blend),
                p.1 + FRAC_PI_2 as f64,
                true,
                "m".to_string() + (i + 1).to_string().as_str(),
            )
        })
        .collect::<Vec<_>>();
    let img = image::io::Reader::new(std::io::Cursor::new(include_bytes!("images\\field.png")))
        .with_guessed_format()?
        .decode()
        .unwrap();
    let size = [img.width() as usize, img.height() as usize];
    let img_buff = img.to_rgba8();
    let pix = img_buff.as_flat_samples();
    Plot::new("Choreo Path")
        .view_aspect((chor::FIELD_X / chor::FIELD_Y) as f32)
        .data_aspect(1.0)
        .cursor_color(Color32::WHITE)
        .show(ui, |plot_ui| {
            plot_ui.ctx().style_mut(|f| {
                f.visuals.override_text_color = Some(egui::Color32::WHITE);
            });
            plot_ui.image(
                PlotImage::new(
                    "bg",
                    ctx.load_texture(
                        "bg_img",
                        ColorImage::from_rgba_unmultiplied(size, pix.as_slice()),
                        Default::default(),
                    )
                    .id(),
                    PlotPoint::new(chor::FIELD_X / 2.0, chor::FIELD_Y / 2.0),
                    [chor::FIELD_X as f32, chor::FIELD_Y as f32],
                )
                .tint(gray_blend2),
            );
            for line in wp_squares {
                plot_ui.line(line);
            }
            for line in wp_mirr_squares {
                plot_ui.line(line);
            }
            for line in sample_segs {
                plot_ui.line(line);
            }
            for line in sample_mirr_segs {
                plot_ui.line(line);
            }
        });
    Ok(())
}

fn draw_rotate_square_rect(
    center: &PlotPoint,
    width: f64,
    height: f64,
    color: egui::Color32,
    angle: f64,
    mirrored: bool,
    name: String,
) -> Line {
    // Compute the half-width and half-height for easier calculations
    let half_width = width / 2.0;
    let half_height = height / 2.0;

    // Define the four corners of the rectangle relative to the center
    let corners = (0..4)
        .map(|i| {
            let (dx, dy) = match i {
                0 => (half_width, half_height),   // Top-right
                1 => (-half_width, half_height),  // Top-left
                2 => (-half_width, -half_height), // Bottom-left
                _ => (half_width, -half_height),  // Bottom-right
            };

            // Apply the rotation matrix
            let rotated_x = center.x + dx * angle.cos() - dy * angle.sin();
            let rotated_y = center.y + dx * angle.sin() + dy * angle.cos();

            [rotated_x, rotated_y]
        })
        .collect::<Vec<[f64; 2]>>(); // Collect points into a vector of f64 arrays

    // Ensure that the shape is closed by adding the first point at the end
    let mut closed_corners = corners.clone();
    closed_corners.push(corners[0]);

    Line::new(name, closed_corners)
        .color(color)
        .style(if mirrored {
            LineStyle::dashed_dense()
        } else {
            LineStyle::Solid
        })
        .fill(center.y as f32)
        .width(if mirrored { 4.0 } else { 4.0 })
}

fn flip_same_alliance(inputfile: String, outputfile: String) -> Result<()> {
    let mut file = File::open(inputfile)?;
    let mut file_out = File::create(outputfile.clone())?;
    let mut data: chor::ChoreoData = serde_json::from_reader(&mut file)?;
    data.snapshot
        .waypoints
        .iter_mut()
        .for_each(chor::ChoreoSWaypoint::flip_same_alliance);
    data.params
        .waypoints
        .iter_mut()
        .for_each(chor::ChoreoWaypoint::flip_same_alliance);
    data.trajectory
        .samples
        .iter_mut()
        .for_each(chor::ChoreoSample::flip_same_alliance);
    data.name = String::from(
        Path::new(&outputfile)
            .file_stem()
            .and_then(|f| f.to_str())
            .unwrap_or(""),
    );
    let new_val = serde_json::to_value(data)?;
    file_out.write_all(
        format_custom(&new_val, false, 0)
            .replace(": ", ":")
            .as_bytes(),
    )?;
    Ok(())
}

fn flip_alliance(inputfile: String, outputfile: String) -> Result<()> {
    let mut file = File::open(inputfile)?;
    let mut file_out = File::create(outputfile.clone())?;
    let mut data: chor::ChoreoData = serde_json::from_reader(&mut file)?;
    data.snapshot
        .waypoints
        .iter_mut()
        .for_each(chor::ChoreoSWaypoint::flip_alliance);
    data.params
        .waypoints
        .iter_mut()
        .for_each(chor::ChoreoWaypoint::flip_alliance);
    data.trajectory
        .samples
        .iter_mut()
        .for_each(chor::ChoreoSample::flip_alliance);
    data.name = String::from(
        Path::new(&outputfile)
            .file_stem()
            .and_then(|f| f.to_str())
            .unwrap_or(""),
    );
    let new_val = serde_json::to_value(data)?;
    file_out.write_all(
        format_custom(&new_val, false, 0)
            .replace(": ", ":")
            .as_bytes(),
    )?;
    Ok(())
}

fn format_custom(value: &serde_json::Value, in_array: bool, indent: usize) -> String {
    match value {
        serde_json::Value::Object(map) => {
            if in_array {
                let entries = map
                    .iter()
                    .map(|(k, v)| format!("\"{}\": {}", k, format_custom(v, true, indent)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{}}}", entries)
            } else {
                let indent_str = " ".repeat(indent);
                let inner_indent_str = " ".repeat(indent + 1);
                let entries = map
                    .iter()
                    .map(|(k, v)| {
                        format!(
                            "{}\"{}\": {}",
                            inner_indent_str,
                            k,
                            format_custom(v, false, indent + 1)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",\n");
                format!("{{\n{}\n{}}}", entries, indent_str)
            }
        }
        serde_json::Value::Array(arr) => {
            if in_array {
                let items: Vec<String> =
                    arr.iter().map(|v| format_custom(v, true, indent)).collect();
                format!("[{}]", items.join(","))
            } else {
                let inner_indent_str = "  ".repeat(indent);
                let all_prim: bool = arr.iter().all(|f| {
                    matches!(
                        f,
                        serde_json::Value::Null
                            | serde_json::Value::Bool(_)
                            | serde_json::Value::Number(_)
                            | serde_json::Value::String(_)
                    )
                });
                let entries = arr
                    .iter()
                    .map(|v| {
                        format!(
                            "{}{}",
                            if all_prim { "" } else { &inner_indent_str },
                            format_custom(v, true, indent + 1)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(if all_prim { "," } else { ",\n" });
                format!("[{}{}]", if all_prim { "" } else { "\n" }, entries)
            }
        }
        _ => value.to_string(),
    }
}
