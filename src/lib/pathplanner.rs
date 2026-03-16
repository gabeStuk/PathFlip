pub mod commands {

    #[derive(serde::Serialize, serde::Deserialize, Clone)]
    #[serde(tag = "type", content = "data")]
    pub enum PPCommand {
        #[serde(rename = "sequential")]
        SequentialCommand { commands: Vec<PPCommand> },
        #[serde(rename = "named")]
        NamedCommand { name: Option<String> },
        #[serde(rename = "path")]
        PathFollowCommand {
            #[serde(rename = "pathName")]
            path_name: Option<String>,
        },
        #[serde(rename = "race")]
        ParallelRaceGroup { commands: Vec<PPCommand> },
        #[serde(rename = "parallel")]
        ParallelCommandGroup { commands: Vec<PPCommand> },
        #[serde(rename = "wait")]
        WaitCommand {
            #[serde(rename = "waitTime")]
            wait_time: f64,
        },
    }

    impl PPCommand {
        pub fn get_command_list(&self) -> Option<&Vec<PPCommand>> {
            match self {
                Self::NamedCommand { name: _ } => Option::None,
                Self::SequentialCommand { commands: c } => Some(&c),
                Self::ParallelCommandGroup { commands: c } => Some(&c),
                Self::ParallelRaceGroup { commands: c } => Some(&c),
                Self::PathFollowCommand { path_name: _ } => Option::None,
                Self::WaitCommand { wait_time: _ } => Option::None,
            }
        }

        pub fn parse_recursive<F>(&self, f: &mut F)
        where
            F: FnMut(&PPCommand),
        {
            if let Some(commands) = self.get_command_list() {
                commands.iter().for_each(|c| c.parse_recursive(f));
            } else {
                f(self);
            }
        }

        pub fn replace_path_commands(&self, names: &Vec<String>) -> Self {
            let mut idx = 0;
            self.replace_path_commands_inner(names, &mut idx)
        }

        fn replace_path_commands_inner(&self, names: &Vec<String>, idx: &mut usize) -> Self {
            match self {
                Self::PathFollowCommand { path_name: _ } => {
                    let new_name = names.get(*idx).cloned();
                    *idx += 1;
                    Self::PathFollowCommand {
                        path_name: new_name,
                    }
                }
                Self::SequentialCommand { commands } => Self::SequentialCommand {
                    commands: commands
                        .iter()
                        .map(|c| c.replace_path_commands_inner(names, idx))
                        .collect(),
                },

                Self::ParallelCommandGroup { commands } => Self::ParallelCommandGroup {
                    commands: commands
                        .iter()
                        .map(|c| c.replace_path_commands_inner(names, idx))
                        .collect(),
                },

                Self::ParallelRaceGroup { commands } => Self::ParallelRaceGroup {
                    commands: commands
                        .iter()
                        .map(|c| c.replace_path_commands_inner(names, idx))
                        .collect(),
                },

                Self::NamedCommand { name } => Self::NamedCommand { name: name.clone() },
                Self::WaitCommand { wait_time } => Self::WaitCommand {
                    wait_time: *wait_time,
                },
            }
        }
    }
}

pub mod auto {

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct AutoData {
        pub version: String,
        pub command: crate::pathplanner::commands::PPCommand,
        #[serde(rename = "resetOdom")]
        pub reset_odom: bool,
        pub folder: Option<String>,
        #[serde(rename = "choreoAuto")]
        pub choreo_auto: bool,
    }

    impl AutoData {
        pub fn get_filenames(&self) -> (Vec<String>, bool) {
            use crate::lib::pathplanner::commands::PPCommand;
            let mut vec: Vec<String> = Vec::new();
            let mut callback = |c: &PPCommand| {
                if let PPCommand::PathFollowCommand { path_name: name } = c {
                    let filename = name.clone().unwrap_or(String::new());
                    vec.push(if let Some(pos) = filename.rfind('.') {
                        if self.choreo_auto {
                            format!("{}.traj", &filename[..pos])
                        } else {
                            format!("{}.path", &filename[..pos])
                        }
                    } else {
                        if self.choreo_auto {
                            format!("{}.traj", &filename)
                        } else {
                            format!("{}.path", &filename)
                        }
                    });
                }
            };

            self.command.parse_recursive(&mut callback);

            vec.sort();
            vec.dedup();

            (vec, self.choreo_auto)
        }
    }
}

pub mod path {
    use crate::lib::flip::{flip_xaxis, flip_yaxis, Flippable};

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct PathPoint {
        pub x: f64,
        pub y: f64,
    }

    impl Flippable for PathPoint {
        fn flip_alliance(&mut self) {
            self.x = flip_yaxis(self.x, self.y)[0];
        }

        fn flip_same_alliance(&mut self) {
            self.y = flip_xaxis(self.x, self.y)[1];
        }
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct PathWaypoint {
        pub anchor: PathPoint,
        #[serde(rename = "prevControl")]
        pub prev_control: Option<PathPoint>,
        #[serde(rename = "nextControl")]
        pub next_control: Option<PathPoint>,
        #[serde(rename = "isLocked")]
        pub is_locked: bool,
        #[serde(rename = "linkedName")]
        pub linked_name: Option<String>,
    }

    impl Flippable for PathWaypoint {
        fn flip_alliance(&mut self) {
            self.anchor.flip_alliance();
            if self.prev_control.is_some() {
                self.prev_control.as_mut().unwrap().flip_alliance();
            }
            if self.next_control.is_some() {
                self.next_control.as_mut().unwrap().flip_alliance();
            }

            if self.linked_name.is_some() {
                self.linked_name =
                    Some(self.linked_name.as_mut().unwrap().to_owned() + " -- flipped");
            }
        }

        fn flip_same_alliance(&mut self) {
            self.anchor.flip_same_alliance();
            if self.prev_control.is_some() {
                self.prev_control.as_mut().unwrap().flip_same_alliance();
            }
            if self.next_control.is_some() {
                self.next_control.as_mut().unwrap().flip_same_alliance();
            }

            if self.linked_name.is_some() {
                self.linked_name =
                    Some(self.linked_name.as_mut().unwrap().to_owned() + " -- flipped");
            }
        }
    }

    #[derive(serde::Serialize, serde::Deserialize, Clone)]
    pub struct PathRotationTarget {
        #[serde(rename = "waypointRelativePos")]
        pub waypoint_relative_pos: f64,
        #[serde(rename = "rotationDegrees")]
        pub rotation_degrees: f64,
    }

    impl Flippable for PathRotationTarget {
        fn flip_alliance(&mut self) {}

        fn flip_same_alliance(&mut self) {
            self.rotation_degrees = if self.rotation_degrees == 0.0 {
                0.0
            } else {
                -self.rotation_degrees
            };
        }
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct PathConstraintZone {
        pub name: String,
        #[serde(rename = "minWaypointRelativePos")]
        pub min_waypoint_relative_pos: f64,
        #[serde(rename = "maxWaypointRelativePos")]
        pub max_waypoint_relative_pos: f64,
        pub constraints: PathConstraints,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct PathPointTowardsZone {
        #[serde(rename = "fieldPosition")]
        field_position: PathPoint,
        #[serde(rename = "rotationOffset")]
        rotation_offset: f64,
        #[serde(rename = "minWaypointRelativePos")]
        pub min_waypoint_relative_pos: f64,
        #[serde(rename = "maxWaypointRelativePos")]
        pub max_waypoint_relative_pos: f64,
        name: String,
    }

    impl Flippable for PathPointTowardsZone {
        fn flip_alliance(&mut self) {}

        fn flip_same_alliance(&mut self) {
            self.field_position.flip_same_alliance();
            self.rotation_offset = if self.rotation_offset == 0.0 {
                0.0
            } else {
                -self.rotation_offset
            };
        }
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct PathEventMarker {
        name: String,
        #[serde(rename = "waypointRelativePos")]
        waypoint_relative_pos: f64,
        #[serde(rename = "endWaypointRelativePos")]
        end_waypoint_relative_pos: Option<f64>,
        command: Option<crate::pathplanner::commands::PPCommand>,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct PathConstraints {
        #[serde(rename = "maxVelocity")]
        pub max_velocity: f64,
        #[serde(rename = "maxAcceleration")]
        pub max_acceleration: f64,
        #[serde(rename = "maxAngularVelocity")]
        pub max_angular_velocity: f64,
        #[serde(rename = "maxAngularAcceleration")]
        pub max_angular_acceleration: f64,
        #[serde(rename = "nominalVoltage")]
        pub nominal_voltage: f64,
        pub unlimited: bool,
    }

    #[derive(serde::Serialize, serde::Deserialize, Clone)]
    pub struct PathGoalState {
        pub velocity: f64,
        pub rotation: f64,
    }

    impl Flippable for PathGoalState {
        fn flip_alliance(&mut self) {}

        fn flip_same_alliance(&mut self) {
            self.rotation = if self.rotation == 0.0 {
                0.0
            } else {
                -self.rotation
            };
        }
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct PathData {
        pub version: String,
        pub waypoints: Vec<PathWaypoint>,
        #[serde(rename = "rotationTargets")]
        pub rotation_targets: Vec<PathRotationTarget>,
        #[serde(rename = "constraintZones")]
        pub constraint_zones: Vec<PathConstraintZone>,
        #[serde(rename = "pointTowardsZones")]
        pub point_towards_zones: Vec<PathPointTowardsZone>,
        #[serde(rename = "eventMarkers")]
        pub event_markers: Vec<PathEventMarker>,
        #[serde(rename = "globalConstraints")]
        pub global_constraints: PathConstraints,
        #[serde(rename = "goalEndState")]
        pub goal_end_state: PathGoalState,
        pub reversed: bool,
        pub folder: Option<String>,
        #[serde(rename = "idealStartingState")]
        pub ideal_starting_state: PathGoalState,
        #[serde(rename = "useDefaultConstraints")]
        pub use_default_constraints: bool,
    }

    impl Flippable for PathData {
        fn flip_alliance(&mut self) {
            self.waypoints.iter_mut().for_each(Flippable::flip_alliance);
            self.rotation_targets
                .iter_mut()
                .for_each(Flippable::flip_alliance);
            self.point_towards_zones
                .iter_mut()
                .for_each(Flippable::flip_alliance);
            self.goal_end_state.flip_alliance();
            self.ideal_starting_state.flip_alliance();
        }

        fn flip_same_alliance(&mut self) {
            self.waypoints
                .iter_mut()
                .for_each(Flippable::flip_same_alliance);
            self.rotation_targets
                .iter_mut()
                .for_each(Flippable::flip_same_alliance);
            self.point_towards_zones
                .iter_mut()
                .for_each(Flippable::flip_same_alliance);
            self.goal_end_state.flip_same_alliance();
            self.ideal_starting_state.flip_same_alliance();
        }
    }
}
