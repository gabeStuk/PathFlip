use std::f64::consts::PI;

use crate::lib::{
    flip::{self, Flippable},
    pathplanner::path::PathPoint,
};

#[derive(Debug, Clone, Copy)]
pub struct Vec2d {
    pub x: f64,
    pub y: f64,
}

impl Vec2d {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn to_array(&self) -> [f64; 2] {
        [self.x, self.y]
    }

    pub fn from_array(xy: [f64; 2]) -> Self {
        Self { x: xy[0], y: xy[1] }
    }

    pub fn from_pathpoint(point: &PathPoint) -> Self {
        Self {
            x: point.x,
            y: point.y,
        }
    }

    pub fn option_from_pathpoint(pointopt: &Option<PathPoint>) -> Option<Self> {
        return if pointopt.is_some() {
            Option::Some(Self::from_pathpoint(pointopt.as_ref().unwrap()))
        } else {
            Option::None
        };
    }

    pub fn add(self, other: Vec2d) -> Vec2d {
        Vec2d::new(self.x + other.x, self.y + other.y)
    }

    pub fn sub(self, other: Vec2d) -> Vec2d {
        Vec2d::new(self.x - other.x, self.y - other.y)
    }

    pub fn scale(self, s: f64) -> Vec2d {
        Vec2d::new(self.x * s, self.y * s)
    }

    pub fn len(self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn norm(self) -> Vec2d {
        let len = self.len();
        if len == 0.0 {
            self
        } else {
            self.scale(1.0 / len)
        }
    }
}

impl Flippable for Vec2d {
    fn flip_alliance(&mut self) {
        self.x = flip::flip_yaxis(self.x, self.y)[0];
    }

    fn flip_same_alliance(&mut self) {
        self.y = flip::flip_xaxis(self.x, self.y)[1];
    }
}

pub mod beizer {
    use crate::lib::{flip::Flippable, util::Vec2d};

    #[derive(Clone)]
    pub struct Anchor {
        pub position: Vec2d,
        pub control_in: Option<Vec2d>,
        pub control_out: Option<Vec2d>,
    }

    impl Flippable for Anchor {
        fn flip_alliance(&mut self) {
            self.position.flip_alliance();
            if self.control_in.is_some() {
                self.control_in.unwrap().flip_alliance();
            }
            if self.control_out.is_some() {
                self.control_out.unwrap().flip_alliance();
            }
        }

        fn flip_same_alliance(&mut self) {
            self.position.flip_same_alliance();
            if self.control_in.is_some() {
                self.control_in.unwrap().flip_same_alliance();
            }
            if self.control_out.is_some() {
                self.control_out.unwrap().flip_same_alliance();
            }
        }
    }

    fn sample_bezier(p0: Vec2d, p1: Vec2d, p2: Vec2d, p3: Vec2d, samples: usize) -> Vec<Vec2d> {
        let mut result = Vec::with_capacity(samples + 1);

        for i in 0..=samples {
            let t = i as f64 / samples as f64;
            let u = 1.0 - t;

            let p = p0
                .scale(u * u * u)
                .add(p1.scale(3.0 * u * u * t))
                .add(p2.scale(3.0 * u * t * t))
                .add(p3.scale(t * t * t));

            result.push(p);
        }

        result
    }

    fn beizer_point(p0: Vec2d, p1: Vec2d, p2: Vec2d, p3: Vec2d, t: f64) -> Vec2d {
        let u = 1.0 - t;
        p0.scale(u * u * u)
            .add(p1.scale(3.0 * u * u * t))
            .add(p2.scale(3.0 * u * t * t))
            .add(p3.scale(t * t * t))
    }

    pub fn point_at(anchors: &Vec<Anchor>, t: f64) -> Vec2d {
        let num_segs = anchors.len() - 1;
        let t_clamped = t.clamp(0.0, num_segs as f64);
        let seg_idx = t_clamped.floor() as usize;

        if seg_idx >= num_segs {
            return anchors.last().unwrap().position;
        }

        let t_local = t_clamped - seg_idx as f64;

        let a0 = &anchors[seg_idx];
        let a1 = &anchors[seg_idx + 1];
        let p0 = a0.position;
        let p3 = a1.position;

        beizer_point(
            p0,
            a0.control_out.unwrap_or(p0),
            a1.control_in.unwrap_or(p3),
            p3,
            t_local,
        )
    }

    pub fn beizer_anchors(anchors: &Vec<Anchor>, samples_per_segment: usize) -> Vec<Vec2d> {
        assert!(anchors.len() >= 2);
        let mut traj = Vec::new();
        for i in 0..anchors.len() - 1 {
            let a0 = &anchors[i];
            let a1 = &anchors[i + 1];
            let p0 = a0.position;
            let p3 = a1.position;
            let segment = sample_bezier(
                p0,
                a0.control_out.unwrap_or(p0),
                a1.control_in.unwrap_or(p3),
                p3,
                samples_per_segment,
            );
            if i > 0 {
                traj.extend_from_slice(&segment[1..]);
            } else {
                traj.extend(segment);
            }
        }

        traj
    }
}

pub fn deg_to_rad(deg: f64) -> f64 {
    return deg * PI / 180.0;
}
