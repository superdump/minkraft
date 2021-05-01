use chrono::{prelude::*, Duration};
use spa::*;

pub struct SolarPosition {
    // in degrees [-90.0, 90.0] where positive is north, negative is south
    pub latitude: f64,
    // [-180.0, 180.0] where positive is east, negative is west
    pub longitude: f64,
    pub simulation_seconds_per_second: f64,
    pub now: DateTime<Utc>,
}

impl SolarPosition {
    pub fn tick(&mut self, t: f64) {
        self.now = self.now
            + Duration::nanoseconds(
                (t * 1_000_000_000f64 * self.simulation_seconds_per_second) as i64,
            );
    }

    pub fn get_azimuth_inclination(&self) -> (f64, f64) {
        let SolarPos {
            azimuth,
            zenith_angle,
        } = calc_solar_position(self.now, self.latitude, self.longitude).unwrap();
        let inclination = 90.0 - zenith_angle;

        (azimuth, inclination)
    }
}

impl Default for SolarPosition {
    fn default() -> Self {
        Self {
            latitude: 0.0,
            longitude: 0.0,
            simulation_seconds_per_second: 1.0,
            now: Utc::now(),
        }
    }
}
