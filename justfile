# swb_carla_world: CARLA behind the world protocol. Its dependency, swb_world_protocol, is a
# submodule under deps/ (git submodule update --init --recursive).

default:
    @just --list

# Build with colcon (the ROS package installs the binary, its launch file and config)
build:
    #!/usr/bin/env bash
    set -e
    unset CARLA_DIR   # carla-sys reads it as a CARLA source tree
    source /opt/ros/humble/setup.bash
    colcon build --symlink-install --base-paths swb_carla_world \
        --cargo-args --profile dev-release

# Rust tests
test:
    env -u CARLA_DIR cargo nextest run --no-tests pass

# Format check and clippy
check:
    cargo fmt --all -- --check
    env -u CARLA_DIR cargo clippy --all-targets -- -D warnings

# Re-measure the blueprint table from a running CARLA (no scenario active)
measure-blueprints port="2000":
    python3 scripts/measure_blueprint_boxes.py --port {{port}}
