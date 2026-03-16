use crate::{flip::Flippable, lib::flip};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ChoreoData {
    pub name: String,
    pub version: i32,
    pub snapshot: ChoreoSnapshotData,
    pub params: ChoreoParams,
    pub trajectory: ChoreoTraj,
    pub events: serde_json::Value,
}

impl Flippable for ChoreoData {
    fn flip_alliance(&mut self) {
        self.snapshot.flip_alliance();
        self.params.flip_alliance();
        self.trajectory.flip_alliance();
    }

    fn flip_same_alliance(&mut self) {
        self.snapshot.flip_same_alliance();
        self.params.flip_same_alliance();
        self.trajectory.flip_same_alliance();
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ChoreoTraj {
    #[serde(rename = "sampleType")]
    pub sample_type: Option<String>,
    pub waypoints: Vec<f64>,
    pub samples: Vec<ChoreoSample>,
    pub splits: Vec<i32>,
}

impl Flippable for ChoreoTraj {
    fn flip_alliance(&mut self) {
        self.samples.iter_mut().for_each(Flippable::flip_alliance);
    }

    fn flip_same_alliance(&mut self) {
        self.samples
            .iter_mut()
            .for_each(Flippable::flip_same_alliance);
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ChoreoSnapshotData {
    pub waypoints: Vec<ChoreoSWaypoint>,
    pub constraints: Vec<ChoreoConstraint>,
    #[serde(rename = "targetDt")]
    pub target_dt: f64,
}

impl Flippable for ChoreoSnapshotData {
    fn flip_alliance(&mut self) {
        self.waypoints.iter_mut().for_each(Flippable::flip_alliance);
    }

    fn flip_same_alliance(&mut self) {
        self.waypoints
            .iter_mut()
            .for_each(Flippable::flip_same_alliance);
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ChoreoParams {
    pub waypoints: Vec<ChoreoWaypoint>,
    pub constraints: Vec<ChoreoConstraint>,
    #[serde(rename = "targetDt")]
    pub target_dt: ChoreoValue,
}

impl Flippable for ChoreoParams {
    fn flip_alliance(&mut self) {
        self.waypoints.iter_mut().for_each(Flippable::flip_alliance);
    }

    fn flip_same_alliance(&mut self) {
        self.waypoints
            .iter_mut()
            .for_each(Flippable::flip_same_alliance);
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ChoreoSWaypoint {
    pub x: f64,
    pub y: f64,
    pub heading: f64,
    pub intervals: i32,
    pub split: bool,
    #[serde(rename = "fixTranslation")]
    pub fix_translation: bool,
    #[serde(rename = "fixHeading")]
    pub fix_heading: bool,
    #[serde(rename = "overrideIntervals")]
    pub override_intervals: bool,
}

impl Flippable for ChoreoSWaypoint {
    fn flip_same_alliance(&mut self) {
        self.y = flip::flip_xaxis(self.x, self.y)[1];
        self.heading = if self.heading == 0.0 {
            0.0
        } else {
            -self.heading
        };
    }

    fn flip_alliance(&mut self) {
        self.x = flip::flip_yaxis(self.x, self.y)[0];
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ChoreoWaypoint {
    pub x: ChoreoValue,
    pub y: ChoreoValue,
    pub heading: ChoreoValue,
    pub intervals: i32,
    pub split: bool,
    #[serde(rename = "fixTranslation")]
    pub fix_translation: bool,
    #[serde(rename = "fixHeading")]
    pub fix_heading: bool,
    #[serde(rename = "overrideIntervals")]
    pub override_intervals: bool,
}

impl Flippable for ChoreoWaypoint {
    fn flip_same_alliance(&mut self) {
        self.y.val = flip::flip_xaxis(self.x.val, self.y.val)[1];
        self.y.update_exp("m");
        self.heading.val = if self.heading.val == 0.0 {
            0.0
        } else {
            -self.heading.val
        };
        self.heading.update_exp("rad");
    }

    fn flip_alliance(&mut self) {
        self.x.val = flip::flip_yaxis(self.x.val, self.y.val)[0];
        self.x.update_exp("m");
        self.heading.update_exp("rad");
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ChoreoSample {
    pub t: f64,
    pub x: f64,
    pub y: f64,
    pub heading: f64,
    pub vx: f64,
    pub vy: f64,
    pub omega: f64,
    pub ax: f64,
    pub ay: f64,
    pub alpha: f64,
    pub fx: Vec<f64>,
    pub fy: Vec<f64>,
}

impl Flippable for ChoreoSample {
    fn flip_same_alliance(&mut self) {
        self.y = flip::flip_xaxis(self.x, self.y)[1];
        self.vy = if self.vy == 0.0 { 0.0 } else { -self.vy };
        self.ay = if self.ay == 0.0 { 0.0 } else { -self.ay };
        self.heading = if self.heading == 0.0 {
            0.0
        } else {
            -self.heading
        };
        self.omega = if self.omega == 0.0 { 0.0 } else { -self.omega };
        self.alpha = if self.alpha == 0.0 { 0.0 } else { -self.alpha };
    }

    fn flip_alliance(&mut self) {
        self.x = flip::flip_yaxis(self.x, self.y)[0];
        self.vx = if self.vx == 0.0 { 0.0 } else { -self.vx };
        self.ax = if self.ax == 0.0 { 0.0 } else { -self.ax };
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ChoreoConstraint {
    pub from: Option<ChoreoWaypointName>,
    pub to: Option<ChoreoWaypointName>,
    pub data: ChoreoConstraintData,
    pub enabled: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ChoreoConstraintData {
    #[serde(rename = "type")]
    pub kind: String,
    pub props: serde_json::Value,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ChoreoValue {
    pub exp: String,
    pub val: f64,
}

impl ChoreoValue {
    pub fn update_exp(&mut self, unit: &str) {
        self.exp = self.val.to_string() + " " + unit;
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum ChoreoWaypointName {
    String(String),
    Int(i32),
}
