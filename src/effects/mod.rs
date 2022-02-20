use std::time::{Duration, Instant};

use crate::api::DeviceLayout;

pub mod mix;
pub mod plasma;
pub mod test;

pub trait Effect {
    fn init(layout: DeviceLayout, num_leds: usize, start_t: Instant) -> Self;
    fn run(&mut self, dt: Option<Duration>) -> Vec<Led>;
}

#[derive(Clone, Default)]
pub struct Led {
    r: f32,
    g: f32,
    b: f32,
}

impl Led {
    pub fn to_gamma_corrected_bytes(&self) -> Vec<u8> {
        let gamma = 2.2;

        let r = f32::powf(self.r, gamma);
        let g = f32::powf(self.g, gamma);
        let b = f32::powf(self.b, gamma);

        vec![
            f32::round(r * 0xFF as f32) as u8,
            f32::round(g * 0xFF as f32) as u8,
            f32::round(b * 0xFF as f32) as u8,
        ]
    }
}
