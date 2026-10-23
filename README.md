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

The CARLA version is the `carla-0916` feature of the `carla` dependency (`Cargo.toml`); a
`CARLA_DIR` in the environment breaks the build (carla-sys reads it as a source tree).
