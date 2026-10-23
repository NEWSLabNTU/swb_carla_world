//! CARLA's weather: its `WeatherParameters` as plain data, CARLA's presets by name, and the
//! world protocol's generic weather in CARLA's terms (roadmap 051: what was the bridge's
//! `swb_interfaces/msg/Weather` lives here, with the CARLA world server).
//!
//! A `set_weather` names a preset (`ClearNoon`, ...; any case, `_`/`-`/spaces ignored) or
//! parameters by [`FIELD_NAMES`]; parameters not named keep CARLA's current values. A reply
//! carries all fourteen fields and the preset they match, if any.

use std::collections::BTreeMap;

use carla::rpc::WeatherParameters;
use swb_core::weather::{Weather as Generic, WeatherRequest};

/// Every field of CARLA 0.9.16's `WeatherParameters`, as plain data.
///
/// The first nine are the ones users set most; the rest ride along so that preset matching
/// sees them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Weather {
    pub cloudiness: f32,
    pub precipitation: f32,
    pub precipitation_deposits: f32,
    pub wind_intensity: f32,
    pub fog_density: f32,
    pub fog_distance: f32,
    pub wetness: f32,
    pub sun_azimuth_angle: f32,
    pub sun_altitude_angle: f32,
    pub fog_falloff: f32,
    pub scattering_intensity: f32,
    pub mie_scattering_scale: f32,
    pub rayleigh_scattering_scale: f32,
    pub dust_storm: f32,
}

impl Weather {
    /// The nine fields users set most (cloud to sun), in [`FIELD_NAMES`] order.
    pub fn user_fields(&self) -> [f32; 9] {
        [
            self.cloudiness,
            self.precipitation,
            self.precipitation_deposits,
            self.wind_intensity,
            self.fog_density,
            self.fog_distance,
            self.wetness,
            self.sun_azimuth_angle,
            self.sun_altitude_angle,
        ]
    }

    /// Equal within what a round trip through CARLA's server preserves.
    fn approx_eq(&self, other: &Self) -> bool {
        let a = self.all();
        let b = other.all();
        a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() <= 1e-3)
    }

    fn all(&self) -> [f32; 14] {
        let u = self.user_fields();
        [
            u[0],
            u[1],
            u[2],
            u[3],
            u[4],
            u[5],
            u[6],
            u[7],
            u[8],
            self.fog_falloff,
            self.scattering_intensity,
            self.mie_scattering_scale,
            self.rayleigh_scattering_scale,
            self.dust_storm,
        ]
    }
}

/// One preset's values in carla-rust's field order (`rpc::weather::wp`), so the rows below
/// read like its table.
#[allow(clippy::too_many_arguments)]
const fn w(
    cloudiness: f32,
    precipitation: f32,
    precipitation_deposits: f32,
    wind_intensity: f32,
    sun_azimuth_angle: f32,
    sun_altitude_angle: f32,
    fog_density: f32,
    fog_distance: f32,
    fog_falloff: f32,
    wetness: f32,
    scattering_intensity: f32,
    mie_scattering_scale: f32,
    rayleigh_scattering_scale: f32,
    dust_storm: f32,
) -> Weather {
    Weather {
        cloudiness,
        precipitation,
        precipitation_deposits,
        wind_intensity,
        fog_density,
        fog_distance,
        wetness,
        sun_azimuth_angle,
        sun_altitude_angle,
        fog_falloff,
        scattering_intensity,
        mie_scattering_scale,
        rayleigh_scattering_scale,
        dust_storm,
    }
}

/// CARLA's weather presets, under their Python API names (`carla.WeatherParameters.<Name>`).
///
/// Values from carla-rust's `rpc::weather`, which reproduces the static presets of CARLA's
/// C++ `WeatherParameters` (LibCarla/source/carla/rpc/WeatherParameters.cpp); the CARLA
/// backend's tests compare every row with carla-rust's. The names keep CARLA's own
/// inconsistencies (`MidRainyNoon` but `MidRainSunset`). `Default` is left out: its all-`-1`
/// values mean "the map's own", which is what an empty `weather` already does.
#[rustfmt::skip]
pub const PRESETS: &[(&str, Weather)] = &[
    //                                cloud  precip dep   wind   azim  alt    fogD   fogDist fogF wet    scatI mieS  raylS   dust
    ("ClearNoon",        w(  5.0,   0.0,  0.0,  10.0, -1.0,  45.0,   2.0,  0.75, 0.1,   0.0, 1.0, 0.03, 0.0331,   0.0)),
    ("CloudyNoon",       w( 60.0,   0.0,  0.0,  10.0, -1.0,  45.0,   3.0,  0.75, 0.1,   0.0, 1.0, 0.03, 0.0331,   0.0)),
    ("WetNoon",          w(  5.0,   0.0, 50.0,  10.0, -1.0,  45.0,   3.0,  0.75, 0.1,   0.0, 1.0, 0.03, 0.0331,   0.0)),
    ("WetCloudyNoon",    w( 60.0,   0.0, 50.0,  10.0, -1.0,  45.0,   3.0,  0.75, 0.1,   0.0, 1.0, 0.03, 0.0331,   0.0)),
    ("SoftRainNoon",     w( 20.0,  30.0, 50.0,  30.0, -1.0,  45.0,   3.0,  0.75, 0.1,   0.0, 1.0, 0.03, 0.0331,   0.0)),
    ("MidRainyNoon",     w( 60.0,  60.0, 60.0,  60.0, -1.0,  45.0,   3.0,  0.75, 0.1,   0.0, 1.0, 0.03, 0.0331,   0.0)),
    ("HardRainNoon",     w(100.0, 100.0, 90.0, 100.0, -1.0,  45.0,   7.0,  0.75, 0.1,   0.0, 1.0, 0.03, 0.0331,   0.0)),
    ("ClearSunset",      w(  5.0,   0.0,  0.0,  10.0, -1.0,  15.0,   2.0,  0.75, 0.1,   0.0, 1.0, 0.03, 0.0331,   0.0)),
    ("CloudySunset",     w( 60.0,   0.0,  0.0,  10.0, -1.0,  15.0,   3.0,  0.75, 0.1,   0.0, 1.0, 0.03, 0.0331,   0.0)),
    ("WetSunset",        w(  5.0,   0.0, 50.0,  10.0, -1.0,  15.0,   2.0,  0.75, 0.1,   0.0, 1.0, 0.03, 0.0331,   0.0)),
    ("WetCloudySunset",  w( 60.0,   0.0, 50.0,  10.0, -1.0,  15.0,   2.0,  0.75, 0.1,   0.0, 1.0, 0.03, 0.0331,   0.0)),
    ("SoftRainSunset",   w( 20.0,  30.0, 50.0,  30.0, -1.0,  15.0,   2.0,  0.75, 0.1,   0.0, 1.0, 0.03, 0.0331,   0.0)),
    ("MidRainSunset",    w( 60.0,  60.0, 60.0,  60.0, -1.0,  15.0,   3.0,  0.75, 0.1,   0.0, 1.0, 0.03, 0.0331,   0.0)),
    ("HardRainSunset",   w(100.0, 100.0, 90.0, 100.0, -1.0,  15.0,   7.0,  0.75, 0.1,   0.0, 1.0, 0.03, 0.0331,   0.0)),
    ("ClearNight",       w(  5.0,   0.0,  0.0,  10.0, -1.0, -90.0,  60.0, 75.0,  1.0,   0.0, 1.0, 0.03, 0.0331,   0.0)),
    ("CloudyNight",      w( 60.0,   0.0,  0.0,  10.0, -1.0, -90.0,  60.0,  0.75, 0.1,   0.0, 1.0, 0.03, 0.0331,   0.0)),
    ("WetNight",         w(  5.0,   0.0, 50.0,  10.0, -1.0, -90.0,  60.0, 75.0,  1.0,  60.0, 1.0, 0.03, 0.0331,   0.0)),
    ("WetCloudyNight",   w( 60.0,   0.0, 50.0,  10.0, -1.0, -90.0,  60.0,  0.75, 0.1,  60.0, 1.0, 0.03, 0.0331,   0.0)),
    ("SoftRainNight",    w( 60.0,  30.0, 50.0,  30.0, -1.0, -90.0,  60.0,  0.75, 0.1,  60.0, 1.0, 0.03, 0.0331,   0.0)),
    ("MidRainyNight",    w( 80.0,  60.0, 60.0,  60.0, -1.0, -90.0,  60.0,  0.75, 0.1,  80.0, 1.0, 0.03, 0.0331,   0.0)),
    ("HardRainNight",    w(100.0, 100.0, 90.0, 100.0, -1.0, -90.0, 100.0,  0.75, 0.1, 100.0, 1.0, 0.03, 0.0331,   0.0)),
    ("DustStorm",        w(100.0,   0.0,  0.0, 100.0, -1.0,  45.0,   2.0,  0.75, 0.1,   0.0, 1.0, 0.03, 0.0331, 100.0)),
];

/// Every preset name, in CARLA's order.
pub fn preset_names() -> impl Iterator<Item = &'static str> {
    PRESETS.iter().map(|(n, _)| *n)
}

/// A preset by name, ignoring case and `_`/`-`/spaces (`clear_noon` finds `ClearNoon`).
/// Returns the canonical name with it.
pub fn preset(name: &str) -> Option<(&'static str, Weather)> {
    let key = normalise(name);
    PRESETS
        .iter()
        .find(|(n, _)| normalise(n) == key)
        .map(|(n, w)| (*n, *w))
}

/// [`preset`], or an error listing the valid names.
pub fn preset_or_err(name: &str) -> Result<(&'static str, Weather), String> {
    preset(name).ok_or_else(|| {
        format!(
            "unknown weather preset '{name}'; one of: {}",
            preset_names().collect::<Vec<_>>().join(", ")
        )
    })
}

/// The preset these values are, if any.
pub fn matching_preset(w: &Weather) -> Option<&'static str> {
    PRESETS
        .iter()
        .find(|(_, p)| p.approx_eq(w))
        .map(|(n, _)| *n)
}

fn normalise(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '_' | '-' | ' '))
        .flat_map(char::to_lowercase)
        .collect()
}

/// The world protocol's names for [`Weather`]'s fields, in its order: the nine users set
/// most first, then the rest.
pub const FIELD_NAMES: [&str; 14] = [
    "cloudiness",
    "precipitation",
    "precipitation_deposits",
    "wind_intensity",
    "fog_density",
    "fog_distance",
    "wetness",
    "sun_azimuth_angle",
    "sun_altitude_angle",
    "fog_falloff",
    "scattering_intensity",
    "mie_scattering_scale",
    "rayleigh_scattering_scale",
    "dust_storm",
];

impl Weather {
    /// Every field, in [`FIELD_NAMES`] order.
    pub fn fields(&self) -> [f32; 14] {
        let u = self.user_fields();
        [
            u[0],
            u[1],
            u[2],
            u[3],
            u[4],
            u[5],
            u[6],
            u[7],
            u[8],
            self.fog_falloff,
            self.scattering_intensity,
            self.mie_scattering_scale,
            self.rayleigh_scattering_scale,
            self.dust_storm,
        ]
    }

    /// From every field, in [`FIELD_NAMES`] order.
    pub fn from_fields(f: [f32; 14]) -> Self {
        Self {
            cloudiness: f[0],
            precipitation: f[1],
            precipitation_deposits: f[2],
            wind_intensity: f[3],
            fog_density: f[4],
            fog_distance: f[5],
            wetness: f[6],
            sun_azimuth_angle: f[7],
            sun_altitude_angle: f[8],
            fog_falloff: f[9],
            scattering_intensity: f[10],
            mie_scattering_scale: f[11],
            rayleigh_scattering_scale: f[12],
            dust_storm: f[13],
        }
    }
}

/// CARLA's weather as [`Weather`].
pub fn from_carla(w: &WeatherParameters) -> Weather {
    {
        Weather {
            cloudiness: w.cloudiness,
            precipitation: w.precipitation,
            precipitation_deposits: w.precipitation_deposits,
            wind_intensity: w.wind_intensity,
            fog_density: w.fog_density,
            fog_distance: w.fog_distance,
            wetness: w.wetness,
            sun_azimuth_angle: w.sun_azimuth_angle,
            sun_altitude_angle: w.sun_altitude_angle,
            fog_falloff: w.fog_falloff,
            scattering_intensity: w.scattering_intensity,
            mie_scattering_scale: w.mie_scattering_scale,
            rayleigh_scattering_scale: w.rayleigh_scattering_scale,
            dust_storm: w.dust_storm,
        }
    }
}

/// [`Weather`] as CARLA's.
pub fn to_carla(w: Weather) -> WeatherParameters {
    let s = w;
    {
        WeatherParameters {
            cloudiness: s.cloudiness,
            precipitation: s.precipitation,
            precipitation_deposits: s.precipitation_deposits,
            wind_intensity: s.wind_intensity,
            sun_azimuth_angle: s.sun_azimuth_angle,
            sun_altitude_angle: s.sun_altitude_angle,
            fog_density: s.fog_density,
            fog_distance: s.fog_distance,
            fog_falloff: s.fog_falloff,
            wetness: s.wetness,
            scattering_intensity: s.scattering_intensity,
            mie_scattering_scale: s.mie_scattering_scale,
            rayleigh_scattering_scale: s.rayleigh_scattering_scale,
            dust_storm: s.dust_storm,
        }
    }
}

/// The weather a request asks for, on top of `current` (CARLA's now), with a label for logs.
/// An unknown preset or parameter name is refused with the valid ones.
pub fn resolve(request: &WeatherRequest, current: Weather) -> Result<(String, Weather), String> {
    match request {
        WeatherRequest::Preset(name) => preset_or_err(name).map(|(name, w)| (name.to_string(), w)),
        WeatherRequest::Parameters(p) => {
            let mut f = current.fields();
            for (name, value) in p {
                let i = FIELD_NAMES.iter().position(|n| n == name).ok_or_else(|| {
                    format!(
                        "unknown weather parameter '{name}'; CARLA's are: {}",
                        FIELD_NAMES.join(", ")
                    )
                })?;
                f[i] = *value as f32;
            }
            Ok(("custom".to_string(), Weather::from_fields(f)))
        }
    }
}

/// `w` as the protocol's generic weather: every field by name, and the preset it matches.
pub fn generic(w: &Weather) -> Generic {
    Generic {
        preset: matching_preset(w).unwrap_or_default().to_string(),
        parameters: FIELD_NAMES
            .iter()
            .zip(w.fields())
            .map(|(n, v)| ((*n).to_string(), f64::from(v)))
            .collect::<BTreeMap<_, _>>(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use carla::rpc::weather as presets;

    #[test]
    fn presets_are_found_by_python_name_and_loosely() {
        let (name, w) = preset("ClearNoon").unwrap();
        assert_eq!(name, "ClearNoon");
        assert_eq!(w.cloudiness, 5.0);
        assert_eq!(w.sun_altitude_angle, 45.0);
        assert_eq!(preset("clearnoon").unwrap().0, "ClearNoon");
        assert_eq!(preset("clear_noon").unwrap().0, "ClearNoon");
        assert_eq!(preset(" Hard-Rain Night").unwrap().0, "HardRainNight");
        assert_eq!(preset("MidRainyNoon").unwrap().0, "MidRainyNoon");
        assert!(preset("Default").is_none());
        assert!(preset("Sunny").is_none());
    }

    #[test]
    fn every_preset_matches_itself_and_only_itself() {
        for name in preset_names() {
            let (_, w) = preset(name).unwrap();
            assert_eq!(matching_preset(&w), Some(name), "{name}");
        }
        assert_eq!(preset_names().count(), 22);
    }

    #[test]
    fn matching_tolerates_float_noise_but_not_a_change() {
        let (_, mut w) = preset("CloudySunset").unwrap();
        w.cloudiness += 1e-5;
        assert_eq!(matching_preset(&w), Some("CloudySunset"));
        w.cloudiness += 1.0;
        assert_eq!(matching_preset(&w), None);
    }

    #[test]
    fn carla_round_trip_keeps_every_field() {
        let (_, w) = preset("WetCloudyNight").unwrap();
        assert_eq!(from_carla(&to_carla(w)), w);
    }

    /// The table is a copy of carla-rust's presets; a carla-rust release that changes
    /// one fails here.
    #[test]
    fn every_preset_is_carla_rusts() {
        let carla: &[(&str, WeatherParameters)] = &[
            ("ClearNoon", presets::CLEAR_NOON),
            ("CloudyNoon", presets::CLOUDY_NOON),
            ("WetNoon", presets::WET_NOON),
            ("WetCloudyNoon", presets::WET_CLOUDY_NOON),
            ("SoftRainNoon", presets::SOFT_RAIN_NOON),
            ("MidRainyNoon", presets::MID_RAIN_NOON),
            ("HardRainNoon", presets::HARD_RAIN_NOON),
            ("ClearSunset", presets::CLEAR_SUNSET),
            ("CloudySunset", presets::CLOUDY_SUNSET),
            ("WetSunset", presets::WET_SUNSET),
            ("WetCloudySunset", presets::WET_CLOUDY_SUNSET),
            ("SoftRainSunset", presets::SOFT_RAIN_SUNSET),
            ("MidRainSunset", presets::MID_RAIN_SUNSET),
            ("HardRainSunset", presets::HARD_RAIN_SUNSET),
            ("ClearNight", presets::CLEAR_NIGHT),
            ("CloudyNight", presets::CLOUDY_NIGHT),
            ("WetNight", presets::WET_NIGHT),
            ("WetCloudyNight", presets::WET_CLOUDY_NIGHT),
            ("SoftRainNight", presets::SOFT_RAIN_NIGHT),
            ("MidRainyNight", presets::MID_RAIN_NIGHT),
            ("HardRainNight", presets::HARD_RAIN_NIGHT),
            ("DustStorm", presets::DUST_STORM),
        ];
        let ours = PRESETS;
        assert_eq!(ours.len(), carla.len());
        for ((name, w), (carla_name, p)) in ours.iter().zip(carla) {
            assert_eq!(name, carla_name);
            assert_eq!(*w, from_carla(p), "{name}");
        }
    }

    #[test]
    fn a_request_resolves_on_top_of_carlas_weather() {
        let (_, now) = preset("DustStorm").unwrap();
        let (label, w) = resolve(&WeatherRequest::Preset("clear_noon".into()), now).unwrap();
        assert_eq!(
            (label.as_str(), w),
            ("ClearNoon", preset("ClearNoon").unwrap().1)
        );
        let p = [
            ("cloudiness".to_string(), 30.0),
            ("sun_altitude_angle".to_string(), 60.0),
        ];
        let (label, w) =
            resolve(&WeatherRequest::Parameters(p.into_iter().collect()), now).unwrap();
        assert_eq!(label, "custom");
        assert_eq!((w.cloudiness, w.sun_altitude_angle), (30.0, 60.0));
        assert_eq!(w.dust_storm, now.dust_storm);
        let e = resolve(&WeatherRequest::Preset("Sunny".into()), now).unwrap_err();
        assert!(
            e.contains("unknown weather preset 'Sunny'") && e.contains("ClearNoon"),
            "{e}"
        );
        let p = [("rain".to_string(), 1.0)];
        let e = resolve(&WeatherRequest::Parameters(p.into_iter().collect()), now).unwrap_err();
        assert!(
            e.contains("unknown weather parameter 'rain'") && e.contains("precipitation"),
            "{e}"
        );
    }

    #[test]
    fn the_generic_form_names_every_field_and_the_preset() {
        let (_, w) = preset("WetCloudyNoon").unwrap();
        let g = generic(&w);
        assert_eq!(g.preset, "WetCloudyNoon");
        assert_eq!(g.parameters.len(), 14);
        assert_eq!(g.parameters["cloudiness"], f64::from(w.cloudiness));
    }
}
