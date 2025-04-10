pub mod chor {
    pub const FIELD_Y: f64 = 8.051902;
    pub const FIELD_X: f64 = 17.54825;
    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct ChoreoData {
        pub name: String,
        pub version: i32,
        pub snapshot: ChoreoSnapshotData,
        pub params: ChoreoParams,
        pub trajectory: ChoreoTraj,
        pub events: serde_json::Value,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct ChoreoTraj {
        #[serde(rename = "sampleType")]
        pub sample_type: Option<String>,
        pub waypoints: Vec<f64>,
        pub samples: Vec<ChoreoSample>,
        pub splits: Vec<i32>,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct ChoreoSnapshotData {
        pub waypoints: Vec<ChoreoSWaypoint>,
        pub constraints: Vec<ChoreoConstraint>,
        #[serde(rename = "targetDt")]
        pub target_dt: f64,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct ChoreoParams {
        pub waypoints: Vec<ChoreoWaypoint>,
        pub constraints: Vec<ChoreoConstraint>,
        #[serde(rename = "targetDt")]
        pub target_dt: ChoreoValue,
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

    impl ChoreoSWaypoint {
        pub fn flip_same_alliance(&mut self) {
            self.y = flip_xaxis(self.x, self.y)[1];
            self.heading = if self.heading == 0.0 {0.0} else {-self.heading};
        }

        pub fn flip_alliance(&mut self) {
            self.x = flip_yaxis(self.x, self.y)[0];
            self.heading = if self.heading == 0.0 {0.0} else {-self.heading};
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

    impl ChoreoWaypoint {
        pub fn flip_same_alliance(&mut self) {
            self.y.val = flip_xaxis(self.x.val, self.y.val)[1];
            self.y.update_exp("m");
            self.heading.val = if self.heading.val == 0.0 {0.0} else {-self.heading.val};
            self.heading.update_exp("rad");
        }

        pub fn flip_alliance(&mut self) {
            self.x.val = flip_yaxis(self.x.val, self.y.val)[0];
            self.x.update_exp("m");
            self.heading.val = if self.heading.val == 0.0 {0.0} else {-self.heading.val};
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

    impl ChoreoSample {
        pub fn flip_same_alliance(&mut self) {
            self.y = flip_xaxis(self.x, self.y)[1];
            self.vy = if self.vy == 0.0 {0.0} else {-self.vy};
            self.ay = if self.ay == 0.0 {0.0} else {-self.ay};
            self.heading = if self.heading == 0.0 {0.0} else {-self.heading};
            self.omega = if self.omega == 0.0 {0.0} else {-self.omega};
            self.alpha = if self.alpha == 0.0 {0.0} else {-self.alpha};
        }

        pub fn flip_alliance(&mut self) {
            self.x = flip_yaxis(self.x, self.y)[0];
            self.vx = if self.vx == 0.0 {0.0} else {-self.vx};
            self.ax = if self.ax == 0.0 {0.0} else {-self.ax};
            self.heading = if self.heading == 0.0 {0.0} else {-self.heading};
            self.omega = if self.omega == 0.0 {0.0} else {-self.omega};
            self.alpha = if self.alpha == 0.0 {0.0} else {-self.alpha};
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

    pub fn flip_xaxis(x: f64, y: f64) -> [f64; 2] {
        [x, FIELD_Y - y]
    }

    pub fn flip_yaxis(x: f64, y: f64) -> [f64; 2] {
        [FIELD_X - x, y]
    }
}