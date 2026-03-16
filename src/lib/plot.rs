use std::{
    fs::{self, File},
    io::{Result, Write},
    path::{Path, PathBuf},
};

use eframe::egui::{self, Color32, ColorImage, TextureHandle};
use egui_plot::{Line, PlotImage, PlotPoint, Points};
use serde::Serialize;
use serde_json::{ser::PrettyFormatter, Serializer, Value};

use crate::{
    lib::{
        chor,
        flip::{self, Flippable},
        pathplanner::{self},
        util::{self, Vec2d},
    },
    FlipFileType,
};

type LinePoints = Vec<Vec<[f64; 2]>>;

pub trait Plotter {
    fn reset(&mut self);
    fn share_bg(&mut self, img: Option<TextureHandle>);
    fn gen(&mut self, filepath: &String, r_xm: f64, r_ym: f64, same_alliance: bool) -> Result<()>;
    fn plot(&mut self, col: &Color32, ctx: &egui::Context, ui: &mut egui::Ui) -> Result<()>;
    fn send_flip(
        &self,
        inputfile: String,
        outputfile: String,
        is_same_alliance: bool,
        auto_file_names: Option<&Vec<String>>,
    ) -> Result<()>;
}

pub struct ChoreoPlotter {
    velocities: Vec<f64>,
    sample_segs: LinePoints,
    sample_mirr_segs: LinePoints,
    wp_squares: LinePoints,
    wp_mirr_squares: LinePoints,
    bg_tex: Option<TextureHandle>,
}

impl Default for ChoreoPlotter {
    fn default() -> Self {
        Self {
            velocities: Vec::new(),
            sample_segs: Vec::new(),
            sample_mirr_segs: Vec::new(),
            wp_squares: Vec::new(),
            wp_mirr_squares: Vec::new(),
            bg_tex: Option::None,
        }
    }
}

impl Plotter for ChoreoPlotter {
    fn reset(&mut self) {
        self.velocities.clear();
        self.sample_segs.clear();
        self.sample_mirr_segs.clear();
        self.wp_squares.clear();
        self.wp_mirr_squares.clear();
    }

    fn share_bg(&mut self, img: Option<TextureHandle>) {
        self.bg_tex = img;
    }

    fn gen(&mut self, filepath: &String, r_ym: f64, r_xm: f64, same_alli: bool) -> Result<()> {
        use std::fs::File;

        let file = File::open(filepath)?;
        let data: chor::ChoreoData = serde_json::from_reader(&file)?;
        let samples = &data.trajectory.samples;
        let waypoints = &data.params.waypoints;

        for pair in samples.windows(2) {
            let s0 = &pair[0];
            let s1 = &pair[1];
            self.sample_segs
                .push(vec![[s0.x as f64, s0.y as f64], [s1.x as f64, s1.y as f64]]);
        }

        self.velocities
            .extend(samples.iter().map(|s| (s.vx * s.vx + s.vy * s.vy).sqrt()));

        let mirr_func = if same_alli {
            flip::flip_xaxis
        } else {
            flip::flip_yaxis
        };
        let mirr_cs = if same_alli {
            Flippable::flip_same_alliance
        } else {
            Flippable::flip_alliance
        };

        for i in 0..samples.len() - 1 {
            let mut smp_window = [samples[i].clone(), samples[i + 1].clone()];
            smp_window.iter_mut().for_each(mirr_cs);
            self.sample_mirr_segs.push(vec![
                [smp_window[0].x, smp_window[0].y],
                [smp_window[1].x, smp_window[1].y],
            ]);
        }

        let wps: Vec<(PlotPoint, f64)> = waypoints
            .iter()
            .map(|wp| {
                (
                    PlotPoint::from([wp.x.val as f64, wp.y.val as f64]),
                    wp.heading.val as f64,
                )
            })
            .collect();

        let mirred_wps: Vec<(PlotPoint, f64)> = wps
            .iter()
            .map(|wp| (PlotPoint::from(mirr_func(wp.0.x, wp.0.y)), -wp.1))
            .collect();

        for (_i, p) in wps.iter().enumerate() {
            self.wp_squares
                .push(draw_rotate_square_rect([p.0.x, p.0.y], r_ym, r_xm, p.1));
        }

        for (_i, p) in mirred_wps.iter().enumerate() {
            self.wp_mirr_squares
                .push(draw_rotate_square_rect([p.0.x, p.0.y], r_ym, r_xm, p.1));
        }

        Ok(())
    }

    fn plot(&mut self, col: &Color32, _: &egui::Context, ui: &mut egui::Ui) -> Result<()> {
        use egui_plot::Plot;
        let gray_blend2 = Color32::from_rgba_unmultiplied(
            Color32::GRAY.r(),
            Color32::GRAY.g(),
            Color32::GRAY.b(),
            150_u8,
        );
        let gray_blend = Color32::from_rgba_unmultiplied(
            Color32::GRAY.r(),
            Color32::GRAY.g(),
            Color32::GRAY.b(),
            25_u8,
        );
        Plot::new("Choreo Path")
            .view_aspect((flip::FIELD_X / flip::FIELD_Y) as f32)
            .data_aspect(1.0)
            .cursor_color(Color32::WHITE)
            .show(ui, |plot_ui| {
                plot_ui.ctx().style_mut(|f| {
                    f.visuals.override_text_color = Some(egui::Color32::WHITE);
                });
                plot_ui.image(PlotImage::new(
                    "bg",
                    self.bg_tex.as_ref().unwrap().id(),
                    PlotPoint::new(flip::FIELD_X / 2.0, flip::FIELD_Y / 2.0),
                    [flip::FIELD_X as f32, flip::FIELD_Y as f32],
                ));
                for pts in &self.wp_squares {
                    let p0 = Vec2d::from_array(pts[0]);
                    let p3 = Vec2d::from_array(pts[3]);
                    plot_ui.points(
                        Points::new(
                            "wp_heading_point",
                            vec![p3.add(p0.sub(p3).scale(0.5)).to_array()],
                        )
                        .color(*col)
                        .radius(8.0),
                    );
                    plot_ui.line(
                        Line::new("wp_square", pts.clone())
                        .color(*col)
                        .style(egui_plot::LineStyle::Solid)
                        .fill((pts.iter().map(|p| p[1]).sum::<f64>() / pts.len() as f64) as f32)
                        .width(4.0),
                    );
                }
                for pts in &self.wp_mirr_squares {
                    let p0 = Vec2d::from_array(pts[0]);
                    let p3 = Vec2d::from_array(pts[3]);
                    plot_ui.points(
                        Points::new(
                            "wp_mirr_heading_point",
                            vec![p3.add(p0.sub(p3).scale(0.5)).to_array()],
                        )
                        .color(col.blend(gray_blend))
                        .radius(8.0),
                    );
                    plot_ui.line(
                        Line::new("wp_mirr_square", pts.clone())
                            .color(col.blend(gray_blend))
                            .style(egui_plot::LineStyle::dashed_dense())
                            .fill((pts.iter().map(|p| p[1]).sum::<f64>() / pts.len() as f64) as f32)
                            .width(4.0),
                    );
                }
                let mut sample_colors: Vec<Color32> = Vec::new();
                let min_vel = self
                    .velocities
                    .iter()
                    .cloned()
                    .fold(f64::INFINITY, f64::min);
                let max_vel = self
                    .velocities
                    .iter()
                    .cloned()
                    .fold(f64::NEG_INFINITY, f64::max);
                let range = (max_vel - min_vel).max(0.01);
                for (i, pts) in self.sample_segs.iter().enumerate() {
                    let avg_vel = (self.velocities[i] + self.velocities[i + 1]) / 2.0;
                    let t = (avg_vel - min_vel) / range;
                    let color = color_lerp(
                        egui::Color32::RED,
                        egui::Color32::GREEN.blend(gray_blend),
                        t,
                    );
                    plot_ui.line(Line::new("sample", pts.clone()).color(color).width(4.0));
                    sample_colors.push(color);
                }
                for (i, pts) in self.sample_mirr_segs.iter().enumerate() {
                    plot_ui.line(
                        Line::new("sample_mirror", pts.clone())
                            .color(sample_colors[i].blend(gray_blend2))
                            .width(4.0),
                    );
                }
            });
        Ok(())
    }

    fn send_flip(
        &self,
        inputfile: String,
        outputfile: String,
        is_same_alliance: bool,
        _: Option<&Vec<String>>,
    ) -> Result<()> {
        let mut file = File::open(inputfile)?;
        let mut file_out = File::create(outputfile.clone())?;
        let mut data: chor::ChoreoData = serde_json::from_reader(&mut file)?;
        data.flip(is_same_alliance);
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
}

pub struct PathplannerPlotter {
    sample_segs: LinePoints,
    sample_mirr_segs: LinePoints,
    rot_targets: LinePoints,
    rot_targets_mirr: LinePoints,
    bg_tex: Option<TextureHandle>,
}

impl Default for PathplannerPlotter {
    fn default() -> Self {
        Self {
            sample_segs: Vec::new(),
            sample_mirr_segs: Vec::new(),
            rot_targets: Vec::new(),
            rot_targets_mirr: Vec::new(),
            bg_tex: Option::None,
        }
    }
}

impl Plotter for PathplannerPlotter {
    fn reset(&mut self) {
        self.sample_segs.clear();
        self.sample_mirr_segs.clear();
        self.rot_targets.clear();
        self.rot_targets_mirr.clear();
    }

    fn share_bg(&mut self, img: Option<TextureHandle>) {
        self.bg_tex = img;
    }

    fn gen(&mut self, filepath: &String, r_ym: f64, r_xm: f64, same_alli: bool) -> Result<()> {
        use std::fs::File;
        let file = File::open(filepath)?;
        let data: pathplanner::path::PathData = serde_json::from_reader(&file)?;
        let goal_start_state = &data.ideal_starting_state;
        let goal_end_state = &data.goal_end_state;
        let mut gs_flipped = goal_start_state.clone();
        gs_flipped.flip(same_alli);
        let mut ge_flipped = goal_end_state.clone();
        ge_flipped.flip(same_alli);
        let control_points = &data.waypoints;
        let rotation_targets = &data.rotation_targets;
        let mut rotation_targets_mirr = rotation_targets.clone();
        rotation_targets_mirr
            .iter_mut()
            .for_each(|rt| rt.flip(same_alli));
        let le_anchors: Vec<util::beizer::Anchor> = control_points
            .iter()
            .map(|pw| util::beizer::Anchor {
                position: Vec2d::from_pathpoint(&pw.anchor),
                control_in: Vec2d::option_from_pathpoint(&pw.prev_control),
                control_out: Vec2d::option_from_pathpoint(&pw.next_control),
            })
            .collect();
        let mut le_anchors_mirr = le_anchors.clone();
        le_anchors_mirr.iter_mut().for_each(|a| a.flip(same_alli));
        let le_samples = util::beizer::beizer_anchors(&le_anchors, 40);
        let mut le_samples_mirr: Vec<Vec2d> = le_samples.clone();
        le_samples_mirr.iter_mut().for_each(|s| s.flip(same_alli));
        for pair in le_samples.windows(2) {
            let s0 = pair[0];
            let s1 = pair[1];
            self.sample_segs.push(vec![s0.to_array(), s1.to_array()]);
        }
        for pair in le_samples_mirr.windows(2) {
            self.sample_mirr_segs
                .push(vec![pair[0].to_array(), pair[1].to_array()])
        }
        let rot_targets_debeizer: Vec<(Vec2d, f64)> = rotation_targets
            .iter()
            .map(|r| {
                (
                    util::beizer::point_at(&le_anchors, r.waypoint_relative_pos),
                    r.rotation_degrees,
                )
            })
            .collect();
        let rot_mirr_debeizer: Vec<(Vec2d, f64)> = rotation_targets_mirr
            .iter()
            .map(|r| {
                (
                    util::beizer::point_at(&le_anchors_mirr, r.waypoint_relative_pos),
                    r.rotation_degrees,
                )
            })
            .collect();
        self.rot_targets.push(draw_rotate_square_rect(
            le_anchors[0].position.to_array(),
            r_xm,
            r_ym,
            util::deg_to_rad(goal_start_state.rotation),
        ));
        for targ in rot_targets_debeizer {
            self.rot_targets.push(draw_rotate_square_rect(
                targ.0.to_array(),
                r_xm,
                r_ym,
                util::deg_to_rad(targ.1),
            ));
        }
        self.rot_targets.push(draw_rotate_square_rect(
            le_anchors.last().unwrap().position.to_array(),
            r_xm,
            r_ym,
            util::deg_to_rad(goal_end_state.rotation),
        ));
        self.rot_targets_mirr.push(draw_rotate_square_rect(
            le_anchors_mirr[0].position.to_array(),
            r_xm,
            r_ym,
            util::deg_to_rad(gs_flipped.rotation),
        ));
        for targ in rot_mirr_debeizer {
            self.rot_targets_mirr.push(draw_rotate_square_rect(
                targ.0.to_array(),
                r_xm,
                r_ym,
                util::deg_to_rad(targ.1),
            ));
        }
        self.rot_targets_mirr.push(draw_rotate_square_rect(
            le_anchors_mirr.last().unwrap().position.to_array(),
            r_xm,
            r_ym,
            util::deg_to_rad(ge_flipped.rotation),
        ));
        Ok(())
    }

    fn plot(&mut self, col: &Color32, _: &egui::Context, ui: &mut egui::Ui) -> Result<()> {
        use egui_plot::Plot;
        let gray_blend2 = Color32::from_rgba_unmultiplied(
            Color32::GRAY.r(),
            Color32::GRAY.g(),
            Color32::GRAY.b(),
            150_u8,
        );
        let gray_blend = Color32::from_rgba_unmultiplied(
            Color32::GRAY.r(),
            Color32::GRAY.g(),
            Color32::GRAY.b(),
            25_u8,
        );
        Plot::new("Pathplanner Path")
            .view_aspect((flip::FIELD_X / flip::FIELD_Y) as f32)
            .data_aspect(1.0)
            .cursor_color(Color32::WHITE)
            .show(ui, |plot_ui| {
                plot_ui.ctx().style_mut(|f| {
                    f.visuals.override_text_color = Some(egui::Color32::WHITE);
                });
                plot_ui.image(PlotImage::new(
                    "bg",
                    self.bg_tex.as_ref().unwrap().id(),
                    PlotPoint::new(flip::FIELD_X / 2.0, flip::FIELD_Y / 2.0),
                    [flip::FIELD_X as f32, flip::FIELD_Y as f32],
                ));
                for pts in &self.rot_targets {
                    let p0 = Vec2d::from_array(pts[0]);
                    let p3 = Vec2d::from_array(pts[3]);
                    plot_ui.points(
                        Points::new(
                            "rot_targ_point",
                            vec![p3.add(p0.sub(p3).scale(0.5)).to_array()],
                        )
                        .color(*col)
                        .radius(8.0),
                    );
                    plot_ui.line(
                        Line::new("rot_targets", pts.clone())
                            .color(*col)
                            .style(egui_plot::LineStyle::Solid)
                            .fill((pts.iter().map(|p| p[1]).sum::<f64>() / pts.len() as f64) as f32)
                            .width(4.0),
                    );
                }
                for pts in &self.rot_targets_mirr {
                    let p0 = Vec2d::from_array(pts[0]);
                    let p3 = Vec2d::from_array(pts[3]);
                    plot_ui.points(
                        Points::new(
                            "mirr_rot_targ_point",
                            vec![p3.add(p0.sub(p3).scale(0.5)).to_array()],
                        )
                        .color(col.blend(gray_blend))
                        .radius(8.0),
                    );
                    plot_ui.line(
                        Line::new("rot_mirr_targets", pts.clone())
                            .color(col.blend(gray_blend))
                            .style(egui_plot::LineStyle::dashed_dense())
                            .fill((pts.iter().map(|p| p[1]).sum::<f64>() / pts.len() as f64) as f32)
                            .width(4.0),
                    );
                }
                for pts in &self.sample_segs {
                    plot_ui.line(
                        Line::new("sample_segs", pts.clone())
                            .color(Color32::BLUE)
                            .width(4.0),
                    );
                }
                for pts in &self.sample_mirr_segs {
                    plot_ui.line(
                        Line::new("sample_mirr_segs", pts.clone())
                            .color(Color32::BLUE.blend(gray_blend2))
                            .width(4.0),
                    );
                }
            });

        Ok(())
    }

    fn send_flip(
        &self,
        inputfile: String,
        outputfile: String,
        is_same_alliance: bool,
        _: Option<&Vec<String>>,
    ) -> Result<()> {
        let mut file = File::open(&inputfile)?;
        let mut file_out = File::create(outputfile.clone())?;
        let mut data: pathplanner::path::PathData = serde_json::from_reader(&mut file)?;
        data.flip(is_same_alliance);
        data.folder = Some("Flipped".to_owned());
        let new_val = serde_json::to_value(data)?;
        file_out.write_all(format_pretty(&new_val).as_bytes())?;
        Self::add_folder(
            Path::new(&inputfile)
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("settings.json"),
            Some("Flipped"),
            None,
        )?;
        Ok(())
    }
}

impl PathplannerPlotter {
    fn send_auto_flip(
        &self,
        inputfile: String,
        outputfile: String,
        auto_file_names: &Vec<String>,
    ) -> Result<()> {
        let mut file = File::open(&inputfile)?;
        let mut file_out = File::create(outputfile.clone())?;
        let mut data: pathplanner::auto::AutoData = serde_json::from_reader(&mut file)?;
        data.folder = Some("Flipped".to_owned());
        data.command = data.command.replace_path_commands(auto_file_names);
        let new_val = serde_json::to_value(data)?;
        file_out.write_all(format_pretty(&new_val).as_bytes())?;
        Self::add_folder(
            Path::new(&inputfile)
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("settings.json"),
            None,
            Some("Flipped"),
        )?;
        Ok(())
    }

    fn add_folder(
        path: PathBuf,
        path_folder: Option<&str>,
        auto_folder: Option<&str>,
    ) -> Result<()> {
        let data = fs::read_to_string(&path)?;
        let mut json: Value = serde_json::from_str(&data)?;

        let mut add_unique = |key: &str, val: &str| {
            if let Some(arr) = json.get_mut(key).and_then(|v| v.as_array_mut()) {
                if !arr.iter().any(|v| v.as_str() == Some(val)) {
                    arr.push(Value::String(val.to_string()));
                }
            }
        };

        if let Some(p) = path_folder {
            add_unique("pathFolders", p);
        }

        if let Some(a) = auto_folder {
            add_unique("autoFolders", a);
        }

        let mut buf = Vec::new();
        let formatter = PrettyFormatter::with_indent(b"    ");
        let mut serializer = Serializer::with_formatter(&mut buf, formatter);
        json.serialize(&mut serializer)?;
        fs::write(path, buf)?;
        Ok(())
    }
}

pub struct DualPlotter {
    pub choreo: ChoreoPlotter,
    pub pathplanner: PathplannerPlotter,
    pub plot_type: FlipFileType,
    pub auto_files: Vec<PathBuf>,
    pub bg_tex: Option<TextureHandle>,
}

impl Default for DualPlotter {
    fn default() -> Self {
        Self {
            choreo: Default::default(),
            pathplanner: Default::default(),
            plot_type: FlipFileType::Choreo,
            auto_files: Vec::new(),
            bg_tex: Option::None,
        }
    }
}

impl Plotter for DualPlotter {
    fn reset(&mut self) {
        self.choreo.reset();
        self.pathplanner.reset();
    }

    fn share_bg(&mut self, _: Option<TextureHandle>) {}

    fn gen(&mut self, filepath: &String, r_xm: f64, r_ym: f64, same_alliance: bool) -> Result<()> {
        match self.plot_type {
            FlipFileType::Choreo => self.choreo.gen(filepath, r_xm, r_ym, same_alliance),
            FlipFileType::Pathplanner => self.pathplanner.gen(filepath, r_xm, r_ym, same_alliance),
            FlipFileType::PathplannerAuto { is_chor: false } => {
                for path in &self.auto_files {
                    self.pathplanner
                        .gen(&path.display().to_string(), r_xm, r_ym, same_alliance)?;
                }

                Ok(())
            }
            FlipFileType::PathplannerAuto { is_chor: true } => {
                for path in &self.auto_files {
                    self.choreo
                        .gen(&path.display().to_string(), r_xm, r_ym, same_alliance)?;
                }

                Ok(())
            }
        }
    }

    fn plot(&mut self, col: &Color32, ctx: &egui::Context, ui: &mut egui::Ui) -> Result<()> {
        if self.bg_tex.is_none() {
            let img = image::ImageReader::new(std::io::Cursor::new(include_bytes!(
                "../images/field.png"
            )))
            .with_guessed_format()?
            .decode()
            .unwrap();
            let size = [img.width() as usize, img.height() as usize];
            let img_buff = img.to_rgba8();
            let pix = img_buff.as_flat_samples();
            self.bg_tex = Some(ctx.load_texture(
                "bg_img",
                ColorImage::from_rgba_unmultiplied(size, pix.as_slice()),
                Default::default(),
            ));
            self.choreo.share_bg(self.bg_tex.clone());
            self.pathplanner.share_bg(self.bg_tex.clone());
        }
        match self.plot_type {
            FlipFileType::Choreo | FlipFileType::PathplannerAuto { is_chor: true } => {
                self.choreo.plot(col, ctx, ui)
            }
            FlipFileType::Pathplanner | FlipFileType::PathplannerAuto { is_chor: false } => {
                self.pathplanner.plot(col, ctx, ui)
            }
        }
    }

    fn send_flip(
        &self,
        inputfile: String,
        outputfile: String,
        is_same_alliance: bool,
        auto_file_names: Option<&Vec<String>>,
    ) -> Result<()> {
        match self.plot_type {
            FlipFileType::Choreo => {
                self.choreo
                    .send_flip(inputfile, outputfile, is_same_alliance, Option::None)
            }
            FlipFileType::Pathplanner => {
                self.pathplanner
                    .send_flip(inputfile, outputfile, is_same_alliance, auto_file_names)
            }
            FlipFileType::PathplannerAuto { is_chor: false } => {
                if let Some(afn_arr) = auto_file_names {
                    for (i, path) in self.auto_files.iter().enumerate() {
                        self.pathplanner.send_flip(
                            path.display().to_string(),
                            self.auto_files[0]
                                .parent()
                                .unwrap()
                                .join(afn_arr[i].clone())
                                .with_extension("path")
                                .display()
                                .to_string(),
                            is_same_alliance,
                            Option::None,
                        )?;
                    }
                }

                self.pathplanner.send_auto_flip(
                    inputfile,
                    outputfile,
                    auto_file_names.unwrap_or(&Vec::new()),
                )
            }
            FlipFileType::PathplannerAuto { is_chor: true } => {
                if let Some(afn_arr) = auto_file_names {
                    for (i, path) in self.auto_files.iter().enumerate() {
                        self.choreo.send_flip(
                            path.display().to_string(),
                            self.auto_files[0]
                                .parent()
                                .unwrap()
                                .join(afn_arr[i].clone())
                                .with_extension("traj")
                                .display()
                                .to_string(),
                            is_same_alliance,
                            Option::None,
                        )?;
                    }
                }

                self.pathplanner.send_auto_flip(
                    inputfile,
                    outputfile,
                    auto_file_names.unwrap_or(&Vec::new()),
                )
            }
        }
    }
}

impl DualPlotter {
    pub fn set_plot_type(&mut self, plot_type: &FlipFileType, paths: Vec<PathBuf>) {
        self.plot_type = *plot_type;
        if paths.len() > 0 {
            self.auto_files = paths;
        }
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

fn draw_rotate_square_rect(center: [f64; 2], width: f64, height: f64, angle: f64) -> Vec<[f64; 2]> {
    let half_width = width / 2.0;
    let half_height = height / 2.0;

    let corners = (0..4)
        .map(|i| {
            let (dx, dy) = match i {
                0 => (half_width, half_height),   // Top-right
                1 => (-half_width, half_height),  // Top-left
                2 => (-half_width, -half_height), // Bottom-left
                _ => (half_width, -half_height),  // Bottom-right
            };

            let rotated_x = center[0] + dx * angle.cos() - dy * angle.sin();
            let rotated_y = center[1] + dx * angle.sin() + dy * angle.cos();

            [rotated_x, rotated_y]
        })
        .collect::<Vec<[f64; 2]>>();

    let mut closed_corners = corners.clone();
    closed_corners.push(corners[0]);

    return closed_corners;
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

fn format_pretty(value: &Value) -> String {
    let mut buf = Vec::new();

    let formatter = PrettyFormatter::with_indent(b"    "); // 4 spaces
    let mut serializer = Serializer::with_formatter(&mut buf, formatter);

    value.serialize(&mut serializer).unwrap();

    String::from_utf8(buf).unwrap()
}
