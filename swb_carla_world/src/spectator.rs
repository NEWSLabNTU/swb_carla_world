//! CARLA's spectator, moved by the world server to follow the ego (roadmap 068 step 2).
//!
//! The world server is the only process that owns the world, so it is the one that can keep
//! the editor's (or the window's) camera over the action: `spectator: follow` places it
//! top-down, `spectator_height` above the target, after every step; `once` places it on the
//! first step after each target spawns; `off` (the default) never touches it, so a headless
//! run does nothing new.
//!
//! The target is the ego; with no ego, the first vehicle spawned, then the first entity of
//! any kind. A despawned target hands the camera to the next by the same rule.
//!
//! Hand-over: a person who grabs the camera keeps it. Before each placement the spectator's
//! pose is compared with the one this server set last; more than [`MOVED_METRES`] or
//! [`MOVED_DEGREES`] apart means someone moved it, and the server leaves it alone until the
//! next `LoadWorld`. The rotation is compared as a rotation, not angle by angle: straight down
//! is UE's gimbal lock, where CARLA may read back the yaw as roll.
//!
//! Everything here is plain data; the two CARLA calls per step (the spectator's
//! `get_transform`, its `set_transform`) are the backend's.

use std::path::Path;
use std::time::Duration;

use eyre::{bail, Result, WrapErr};
use serde::Deserialize;

/// More than this between the pose read back and the pose last set: a person moved it.
pub const MOVED_METRES: f64 = 0.1;
/// More than this, the same.
pub const MOVED_DEGREES: f64 = 1.0;

/// `spectator` in `swb_carla_world.yaml`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Never touch the spectator.
    #[default]
    Off,
    /// Once per target, on the first step after it spawns.
    Once,
    /// After every step.
    Follow,
}

/// The spectator's keys of `swb_carla_world.yaml`. They are the CARLA world server's alone,
/// so they are read here rather than in the bridge's shared `BridgeConfig` (which ignores
/// them).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(default)]
pub struct Config {
    pub spectator: Mode,
    /// Metres above the target.
    pub spectator_height: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            spectator: Mode::Off,
            spectator_height: 50.0,
        }
    }
}

impl Config {
    /// The two keys of `text` (a whole `swb_carla_world.yaml`); every other key is ignored.
    pub fn parse(text: &str) -> Result<Self> {
        // An empty file is an empty document, not a mapping.
        if text.trim().is_empty() {
            return Ok(Self::default());
        }
        let config: Self = serde_yaml::from_str(text)?;
        if !(config.spectator_height.is_finite() && config.spectator_height > 0.0) {
            bail!(
                "spectator_height: {} is not a height above the target (metres, > 0)",
                config.spectator_height
            );
        }
        Ok(config)
    }

    /// The keys from the file `path`; a missing file is the defaults (as for the rest of the
    /// file), a malformed one is an error.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text =
            std::fs::read_to_string(path).wrap_err_with(|| format!("read {}", path.display()))?;
        Self::parse(&text).wrap_err_with(|| format!("parse {}", path.display()))
    }
}

/// What an entity is to the target rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The entity whose motion is the ego's (the vehicle stack's).
    Ego,
    /// Any other vehicle.
    Vehicle,
    /// A pedestrian, a misc object.
    Other,
}

/// A CARLA transform as plain numbers: metres, and degrees as UE's `FRotator` names them.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct View {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub pitch: f64,
    pub yaw: f64,
    pub roll: f64,
}

impl View {
    /// Straight down on `target`, `height` above it. The yaw puts CARLA's -y (the map's +y)
    /// at the top of the picture and +x to the right, as RViz shows the lanelet map; it does
    /// not turn with the target, so the picture only slides.
    pub fn top_down(target: &View, height: f64) -> Self {
        Self {
            x: target.x,
            y: target.y,
            z: target.z + height,
            pitch: -90.0,
            yaw: -90.0,
            roll: 0.0,
        }
    }

    /// Straight-line distance between the two positions, m.
    pub fn distance(&self, other: &View) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2) + (self.z - other.z).powi(2))
            .sqrt()
    }

    /// The angle of the rotation from one orientation to the other, degrees: the same for
    /// every (pitch, yaw, roll) that names the same orientation, gimbal lock included.
    pub fn angle_to(&self, other: &View) -> f64 {
        let a = self.axes();
        let b = other.axes();
        // trace(A Bᵀ) = 1 + 2 cos θ, with A's and B's rows the rotated unit axes.
        let trace: f64 = (0..3)
            .map(|i| (0..3).map(|j| a[i][j] * b[i][j]).sum::<f64>())
            .sum();
        ((trace - 1.0) / 2.0).clamp(-1.0, 1.0).acos().to_degrees()
    }

    /// UE's `FRotationMatrix`: its rows are the rotated forward, right and up axes.
    fn axes(&self) -> [[f64; 3]; 3] {
        let (sp, cp) = self.pitch.to_radians().sin_cos();
        let (sy, cy) = self.yaw.to_radians().sin_cos();
        let (sr, cr) = self.roll.to_radians().sin_cos();
        [
            [cp * cy, cp * sy, sp],
            [sr * sp * cy - cr * sy, sr * sp * sy + cr * cy, -sr * cp],
            [-(cr * sp * cy + sr * sy), cy * sr - cr * sp * sy, cr * cp],
        ]
    }

    /// Whether a spectator read back at `self` is no longer where it was set (`set`): a
    /// person moved it.
    pub fn moved_from(&self, set: &View) -> bool {
        self.distance(set) > MOVED_METRES || self.angle_to(set) > MOVED_DEGREES
    }
}

/// What to do with the spectator this step, given its pose and the target's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Decision {
    /// Set it here.
    Place(View),
    /// A person moved it: leave it alone until the next `LoadWorld` (and say so, once).
    HandOver { moved_by: f64, turned_by: f64 },
}

/// The spectator's state for one world: the candidates, the target, the last pose set.
#[derive(Debug)]
pub struct Spectator {
    config: Config,
    /// Every entity alive, in spawn order.
    entities: Vec<(String, Role)>,
    /// The pose this server set last in this world.
    last_set: Option<View>,
    /// `once`: the target it was placed over (a new target is placed over once more).
    placed_over: Option<String>,
    /// A person took the camera: nothing more until the next `LoadWorld`.
    handed_over: bool,
    /// A CARLA call for the spectator failed: logged once per world.
    warned: bool,
    /// Placements this world, and their time in CARLA calls.
    placements: u32,
    cost: Duration,
    worst: Duration,
}

impl Spectator {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            entities: Vec::new(),
            last_set: None,
            placed_over: None,
            handed_over: false,
            warned: false,
            placements: 0,
            cost: Duration::ZERO,
            worst: Duration::ZERO,
        }
    }

    /// A new world (`LoadWorld`): no entities, nothing set, and a camera a person took is
    /// the server's again. Returns the last world's cost line, if it placed anything.
    pub fn new_world(&mut self) -> Option<String> {
        let summary = self.summary();
        *self = Self::new(self.config);
        summary
    }

    pub fn spawned(&mut self, name: &str, role: Role) {
        self.entities.retain(|(n, _)| n != name);
        self.entities.push((name.to_string(), role));
    }

    pub fn despawned(&mut self, name: &str) {
        self.entities.retain(|(n, _)| n != name);
    }

    /// The ego; else the first vehicle spawned; else the first entity.
    pub fn target(&self) -> Option<&str> {
        let first = |role: Role| self.entities.iter().find(|(_, r)| *r == role);
        first(Role::Ego)
            .or_else(|| first(Role::Vehicle))
            .or_else(|| self.entities.first())
            .map(|(n, _)| n.as_str())
    }

    /// The entity to place the camera over this step, if a placement is due.
    pub fn due(&self) -> Option<&str> {
        if self.handed_over {
            return None;
        }
        let target = self.target()?;
        match self.config.spectator {
            Mode::Off => None,
            Mode::Follow => Some(target),
            Mode::Once => (self.placed_over.as_deref() != Some(target)).then_some(target),
        }
    }

    /// Given the spectator's pose now (`current`) and the target's, place it or let it go.
    /// [`Spectator::placed`] confirms a `Place` once CARLA took it.
    pub fn decide(&mut self, current: &View, target: &View) -> Decision {
        if let Some(set) = self.last_set {
            if current.moved_from(&set) {
                self.handed_over = true;
                return Decision::HandOver {
                    moved_by: current.distance(&set),
                    turned_by: current.angle_to(&set),
                };
            }
        }
        Decision::Place(View::top_down(target, self.config.spectator_height))
    }

    /// CARLA took `view` for `target`; `cost` is the time spent in the two CARLA calls.
    pub fn placed(&mut self, target: &str, view: View, cost: Duration) {
        self.last_set = Some(view);
        self.placed_over = Some(target.to_string());
        self.placements += 1;
        self.cost += cost;
        self.worst = self.worst.max(cost);
    }

    /// A CARLA call for the spectator failed: true the first time this world, to log it.
    pub fn failed(&mut self) -> bool {
        !std::mem::replace(&mut self.warned, true)
    }

    /// Placements and their cost so far this world, as a log line; `None` before the first.
    pub fn summary(&self) -> Option<String> {
        (self.placements > 0).then(|| {
            let ms = |d: Duration| d.as_secs_f64() * 1e3;
            format!(
                "Spectator: {} placement(s), mean {:.2} ms, max {:.2} ms (get_transform + \
                 set_transform){}",
                self.placements,
                ms(self.cost) / self.placements as f64,
                ms(self.worst),
                if self.handed_over {
                    "; handed over to a person"
                } else {
                    ""
                }
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: f64, y: f64, z: f64) -> View {
        View {
            x,
            y,
            z,
            ..Default::default()
        }
    }

    fn follow(height: f64) -> Spectator {
        Spectator::new(Config {
            spectator: Mode::Follow,
            spectator_height: height,
        })
    }

    #[test]
    fn the_keys_default_to_off_at_fifty_metres() {
        let c = Config::parse("ego:\n  role_name: hero\nsubsteps: 2\n").unwrap();
        assert_eq!(c, Config::default());
        assert_eq!(c.spectator, Mode::Off);
        assert_eq!(c.spectator_height, 50.0);
        assert_eq!(Config::parse("").unwrap(), Config::default());
        assert_eq!(
            Config::parse("# only a comment\n").unwrap(),
            Config::default()
        );
    }

    #[test]
    fn the_keys_parse_every_mode_and_a_height() {
        let c = Config::parse("spectator: follow\nspectator_height: 30\n").unwrap();
        assert_eq!(c.spectator, Mode::Follow);
        assert_eq!(c.spectator_height, 30.0);
        assert_eq!(
            Config::parse("spectator: once\n").unwrap().spectator,
            Mode::Once
        );
        // YAML 1.2: `off` is a string, not false.
        assert_eq!(
            Config::parse("spectator: off\n").unwrap().spectator,
            Mode::Off
        );
    }

    #[test]
    fn a_bad_mode_or_height_is_refused() {
        assert!(Config::parse("spectator: chase\n").is_err());
        assert!(Config::parse("spectator_height: 0\n").is_err());
        assert!(Config::parse("spectator_height: -5\n").is_err());
        assert!(Config::parse("spectator_height: .nan\n").is_err());
    }

    #[test]
    fn a_missing_file_is_the_defaults() {
        let c = Config::load(Path::new("/nonexistent/swb_carla_world.yaml")).unwrap();
        assert_eq!(c, Config::default());
    }

    #[test]
    fn the_target_is_the_ego_then_the_first_vehicle_then_the_first_entity() {
        let mut s = follow(50.0);
        assert_eq!(s.target(), None);
        s.spawned("walker", Role::Other);
        assert_eq!(s.target(), Some("walker"));
        s.spawned("npc1", Role::Vehicle);
        s.spawned("npc2", Role::Vehicle);
        assert_eq!(s.target(), Some("npc1"));
        s.spawned("ego", Role::Ego);
        assert_eq!(s.target(), Some("ego"));
        // A despawned target hands the camera to the next.
        s.despawned("ego");
        assert_eq!(s.target(), Some("npc1"));
        s.despawned("npc1");
        assert_eq!(s.target(), Some("npc2"));
        s.despawned("npc2");
        assert_eq!(s.target(), Some("walker"));
        s.despawned("walker");
        assert_eq!(s.target(), None);
    }

    #[test]
    fn off_never_places() {
        let mut s = Spectator::new(Config::default());
        s.spawned("ego", Role::Ego);
        assert_eq!(s.due(), None);
    }

    #[test]
    fn top_down_is_straight_down_at_the_height() {
        let v = View::top_down(
            &View {
                x: 10.0,
                y: -20.0,
                z: 0.5,
                pitch: 3.0,
                yaw: 135.0,
                roll: 1.0,
            },
            50.0,
        );
        assert_eq!((v.x, v.y, v.z), (10.0, -20.0, 50.5));
        assert_eq!((v.pitch, v.yaw, v.roll), (-90.0, -90.0, 0.0));
        // Its forward axis points down.
        let f = v.axes()[0];
        assert!(f[0].abs() < 1e-9 && f[1].abs() < 1e-9 && (f[2] + 1.0).abs() < 1e-9);
    }

    #[test]
    fn follow_places_every_step_over_the_moving_target() {
        let mut s = follow(40.0);
        s.spawned("ego", Role::Ego);
        let mut current = at(0.0, 0.0, 100.0); // wherever CARLA left it
        for i in 0..5 {
            assert_eq!(s.due(), Some("ego"));
            let target = at(i as f64 * 1.5, 2.0, 0.3);
            let Decision::Place(v) = s.decide(&current, &target) else {
                panic!("step {i}: handed over");
            };
            assert_eq!((v.x, v.y, v.z), (target.x, target.y, 40.3));
            s.placed("ego", v, Duration::from_micros(200));
            current = v; // CARLA reads back what was set
        }
        assert!(s
            .summary()
            .unwrap()
            .starts_with("Spectator: 5 placement(s)"));
    }

    #[test]
    fn once_places_once_per_target() {
        let mut s = Spectator::new(Config {
            spectator: Mode::Once,
            spectator_height: 50.0,
        });
        assert_eq!(s.due(), None);
        s.spawned("npc", Role::Vehicle);
        assert_eq!(s.due(), Some("npc"));
        let Decision::Place(v) = s.decide(&at(0.0, 0.0, 0.0), &at(5.0, 5.0, 0.0)) else {
            panic!("handed over");
        };
        s.placed("npc", v, Duration::ZERO);
        assert_eq!(s.due(), None);
        // The ego arrives: it is the target now, placed over once.
        s.spawned("ego", Role::Ego);
        assert_eq!(s.due(), Some("ego"));
        let Decision::Place(w) = s.decide(&v, &at(9.0, 9.0, 0.0)) else {
            panic!("handed over");
        };
        s.placed("ego", w, Duration::ZERO);
        assert_eq!(s.due(), None);
    }

    #[test]
    fn a_camera_a_person_moved_is_left_alone_until_the_next_world() {
        let mut s = follow(50.0);
        s.spawned("ego", Role::Ego);
        let target = at(0.0, 0.0, 0.0);
        let Decision::Place(v) = s.decide(&at(0.0, 0.0, 0.0), &target) else {
            panic!("handed over");
        };
        s.placed("ego", v, Duration::ZERO);
        // Within the tolerance: still ours.
        let nudged = View {
            x: v.x + 0.05,
            yaw: v.yaw + 0.5,
            ..v
        };
        assert!(matches!(s.decide(&nudged, &target), Decision::Place(_)));
        // 2 m away: a person moved it.
        let moved = View { x: v.x + 2.0, ..v };
        match s.decide(&moved, &target) {
            Decision::HandOver { moved_by, .. } => assert!((moved_by - 2.0).abs() < 1e-9),
            d => panic!("{d:?}"),
        }
        assert_eq!(s.due(), None);
        assert!(s.summary().unwrap().ends_with("handed over to a person"));
        // The next LoadWorld gives it back.
        assert!(s.new_world().is_some());
        assert_eq!(s.due(), None, "no entities yet");
        s.spawned("ego", Role::Ego);
        assert_eq!(s.due(), Some("ego"));
        assert!(matches!(s.decide(&moved, &target), Decision::Place(_)));
    }

    #[test]
    fn turning_it_counts_as_moving_it() {
        let mut s = follow(50.0);
        s.spawned("ego", Role::Ego);
        let Decision::Place(v) = s.decide(&at(0.0, 0.0, 0.0), &at(0.0, 0.0, 0.0)) else {
            panic!("handed over");
        };
        s.placed("ego", v, Duration::ZERO);
        let tilted = View {
            pitch: v.pitch + 1.5,
            ..v
        };
        assert!(matches!(
            s.decide(&tilted, &at(0.0, 0.0, 0.0)),
            Decision::HandOver { .. }
        ));
    }

    #[test]
    fn rotations_are_compared_as_rotations_through_the_gimbal_lock() {
        // Straight down, yaw and roll are one degree of freedom: (-90, 30, 0) and (-90, 0, 30)
        // name the same orientation, which CARLA may read back either way.
        let a = View {
            pitch: -90.0,
            yaw: 30.0,
            ..Default::default()
        };
        let b = View {
            pitch: -90.0,
            roll: 30.0,
            ..Default::default()
        };
        assert!(a.angle_to(&b) < 1e-6);
        assert!(!b.moved_from(&a));
        let c = View { yaw: 31.5, ..a };
        assert!((a.angle_to(&c) - 1.5).abs() < 1e-6);
        assert!(c.moved_from(&a));
        // Away from the lock, angles add as expected; -180 and 180 are one yaw.
        let level = View {
            yaw: 180.0,
            ..Default::default()
        };
        let level2 = View {
            yaw: -180.0,
            ..Default::default()
        };
        assert!(level.angle_to(&level2) < 1e-6);
        let d = View {
            yaw: 90.0,
            ..Default::default()
        };
        assert!((level.angle_to(&d) - 90.0).abs() < 1e-6);
    }

    #[test]
    fn a_failure_is_reported_once_per_world() {
        let mut s = follow(50.0);
        assert!(s.failed());
        assert!(!s.failed());
        s.new_world();
        assert!(s.failed());
    }
}
