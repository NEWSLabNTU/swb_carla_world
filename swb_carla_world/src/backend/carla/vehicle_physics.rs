//! The spec's physics on a CARLA vehicle (roadmap 051 K3): sim_vehicle_spec's
//! `carla_physics.yaml` onto `VehiclePhysicsControl`, and the read-back that proves it took.
//! The world server applies it to every vehicle a vehicle stack drives (the ego, `agent`
//! vehicles) whose body is the file's blueprint; acb_bridge reads it back on attach.

use carla::client::Vehicle;
use carla::geom::{Location, Vector2D};
use carla::rpc::{GearPhysicsControl, VehiclePhysicsControl};
use carla_physics_spec::{CarlaPhysics, ReadBack};

/// `control` with the file's physics: what it says, the rest of the blueprint's kept.
pub fn set(control: &mut VehiclePhysicsControl, p: &CarlaPhysics) {
    let v = &p.vehicle;
    control.mass = v.mass as f32;
    control.center_of_mass = Location {
        x: v.center_of_mass[0] as f32,
        y: v.center_of_mass[1] as f32,
        z: v.center_of_mass[2] as f32,
    };
    control.drag_coefficient = v.drag_coefficient as f32;
    control.max_rpm = v.max_rpm as f32;
    control.torque_curve = v
        .torque_curve
        .iter()
        .map(|[rpm, nm]| Vector2D {
            x: *rpm as f32,
            y: *nm as f32,
        })
        .collect();
    control.final_ratio = v.final_ratio as f32;
    control.forward_gears = v
        .forward_gears
        .iter()
        .map(|g| GearPhysicsControl {
            ratio: g.ratio as f32,
            down_ratio: g.down_ratio as f32,
            up_ratio: g.up_ratio as f32,
        })
        .collect();
    control.use_gear_autobox = v.use_gear_autobox;
    control.moi = v.moi as f32;
    control.damping_rate_full_throttle = v.damping_rate_full_throttle as f32;
    control.damping_rate_zero_throttle_clutch_engaged =
        v.damping_rate_zero_throttle_clutch_engaged as f32;
    control.damping_rate_zero_throttle_clutch_disengaged =
        v.damping_rate_zero_throttle_clutch_disengaged as f32;
    for (wheel, w) in control.wheels.iter_mut().zip(&p.wheels) {
        wheel.tire_friction = w.tire_friction as f32;
        wheel.damping_rate = w.damping_rate as f32;
        wheel.max_steer_angle = w.max_steer_angle_deg as f32;
        wheel.radius = (w.radius_m * 100.0) as f32; // CARLA: cm
        wheel.max_brake_torque = w.max_brake_torque as f32;
        wheel.max_handbrake_torque = w.max_handbrake_torque as f32;
        wheel.lat_stiff_max_load = w.lat_stiff_max_load as f32;
        wheel.lat_stiff_value = w.lat_stiff_value as f32;
        wheel.long_stiff_value = w.long_stiff_value as f32;
    }
}

/// What an actor's physics control says, in the file's units.
pub fn read_back(control: &VehiclePhysicsControl) -> ReadBack {
    ReadBack {
        mass: f64::from(control.mass),
        center_of_mass: [
            f64::from(control.center_of_mass.x),
            f64::from(control.center_of_mass.y),
            f64::from(control.center_of_mass.z),
        ],
        drag_coefficient: f64::from(control.drag_coefficient),
        max_rpm: f64::from(control.max_rpm),
        final_ratio: f64::from(control.final_ratio),
        peak_torque: control
            .torque_curve
            .iter()
            .map(|p| f64::from(p.y))
            .fold(0.0, f64::max),
        moi: f64::from(control.moi),
        damping_rate_zero_throttle_clutch_engaged: f64::from(
            control.damping_rate_zero_throttle_clutch_engaged,
        ),
        wheels: control
            .wheels
            .iter()
            .map(|w| {
                [
                    f64::from(w.tire_friction),
                    f64::from(w.radius) / 100.0,
                    f64::from(w.max_brake_torque),
                    f64::from(w.max_steer_angle),
                    f64::from(w.long_stiff_value),
                ]
            })
            .collect(),
    }
}

/// Put the file's physics on `vehicle`. CARLA applies it at the next tick: [`check`] after.
pub fn apply(vehicle: &Vehicle, p: &CarlaPhysics) -> eyre::Result<()> {
    let mut control = vehicle
        .physics_control()
        .map_err(|e| eyre::eyre!("read its physics: {e}"))?;
    if control.wheels.len() != p.wheels.len() {
        eyre::bail!(
            "it has {} wheels, the physics {}",
            control.wheels.len(),
            p.wheels.len()
        );
    }
    set(&mut control, p);
    vehicle
        .apply_physics_control(&control)
        .map_err(|e| eyre::eyre!("apply the physics: {e}"))
}

/// After a tick: the physics `vehicle` runs is the file's. An error names what is not.
pub fn check(vehicle: &Vehicle, p: &CarlaPhysics) -> eyre::Result<()> {
    let back = vehicle
        .physics_control()
        .map_err(|e| eyre::eyre!("read the physics back: {e}"))?;
    let off = p.differences(&read_back(&back), 1.0);
    if !off.is_empty() {
        eyre::bail!("the physics did not take: {}", off.join("; "));
    }
    Ok(())
}
