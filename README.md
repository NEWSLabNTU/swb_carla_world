# swb_carla_world

[CARLA](https://carla.org/) 0.9.16 behind the world protocol of
[scenario-world-bridge](https://github.com/NEWSLabNTU/scenario-world-bridge): the CARLA
**world server**. It loads the town, spawns and moves the scenario's vehicles and people
(SSv2's NPCs teleported or tracked with physics, Traffic Manager autopilots, Autoware-driven
`agent` vehicles), sets traffic lights, weather and road friction, ticks CARLA synchronously
and reports contacts. It is the only code of the project that uses carla-rust.

One server per CARLA: it holds `/tmp/swb-locks/carla-<host>-<port>.lock` for its whole
life, and exits 75 for its supervisor when CARLA goes away.

```bash
git submodule update --init --recursive   # deps/swb_world_protocol
just build
play_launch launch swb_carla_world swb_carla_world.launch.xml port:=5591 carla_port:=2000
swb_world_check conformance tcp://127.0.0.1:5591 --map <map dir>   # from swb_world_protocol
```

| Path | What |
|---|---|
| `swb_carla_world/` | The ROS package (ament_cargo): the server, `launch/swb_carla_world.launch.xml`, `config/swb_carla_world.yaml`, the measured blueprint table `config/blueprint_boxes_0.9.16.yaml` |
| `scripts/measure_blueprint_boxes.py` | Re-measures that table (boxes and axles) from a running CARLA |
| `deps/swb_world_protocol` | The protocol and its Rust shell (`swb_core`), a submodule |
| `deps/sim_vehicle_spec` | The vehicle specs; the ego's `carla_physics.yaml` comes from here, a submodule |

The CARLA version is the `carla-0916` feature of the `carla` dependency (`Cargo.toml`); a
`CARLA_DIR` in the environment breaks the build (carla-sys reads it as a source tree).

## The ego's physics

A scenario's ego runs the car its vehicle spec describes (roadmap 051, K3): the server
applies `carla_physics.yaml` (ROS parameter `vehicle_physics`, default the installed
`sim_vehicle_spec/generated/tesla_model3/carla_physics.yaml`; `none` leaves the blueprint's)
before the spawn's settling tick, reads it back after that tick and refuses an actor that
differs. The spawn reply names the spec's `spec_hash`, which the bridge compares with the
vehicle side's.

## Time and episodes

Simulation time is `nanos(elapsed_seconds) + BASE_NS` (`swb_core::sim_time`); the episode is
CARLA's world id, in `LoadWorldReply.episode_id` and `StepReply.episode_id`. A world load
starts a new episode, and every vehicle stack of the old one ends (acb_bridge exits).

## Weather

`SetWeather` takes a CARLA preset by name (`ClearNoon`, `CloudySunset`, `HardRainNight`, ...;
any case, `_`/`-`/spaces ignored) or CARLA's `WeatherParameters` fields by name; fields not
named keep their current values. `GetWeather` returns all fourteen and the preset they
match. Through the bridge:

```bash
ros2 service call /scenario_world_bridge/set_weather swb_interfaces/srv/SetWeather "{preset: ClearNoon}"
ros2 service call /scenario_world_bridge/set_weather swb_interfaces/srv/SetWeather \
    "{names: [cloudiness, sun_altitude_angle], values: [30.0, 60.0]}"
```

| field | range |
|---|---|
| `cloudiness`, `precipitation` (rain), `precipitation_deposits` (puddles), `wind_intensity`, `fog_density`, `wetness` | 0-100 |
| `fog_distance` | metres where fog starts |
| `sun_azimuth_angle` | degrees, 0-360 |
| `sun_altitude_angle` | degrees, -90 (midnight) to 90 (midday) |
| `fog_falloff`, `scattering_intensity`, `mie_scattering_scale`, `rayleigh_scattering_scale`, `dust_storm` | as CARLA's [`WeatherParameters`](https://carla.readthedocs.io/en/0.9.16/python_api/#carla.WeatherParameters) |

The server keeps the weather asked for -- `weather:` in `swb_carla_world.yaml` (checked at
start), then the last `SetWeather` -- and applies it again after every world load and
reconnect.
