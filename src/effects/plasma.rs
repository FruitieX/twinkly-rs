use std::{
    f32::consts::PI,
    time::{Duration, Instant},
};

use crate::api::DeviceLayout;

use super::{Effect, Led};

pub struct Plasma {
    layout: DeviceLayout,
    num_leds: usize,
    start_t: Instant,
}

impl Effect for Plasma {
    fn init(layout: DeviceLayout, num_leds: usize, start_t: Instant) -> Self {
        Self {
            layout,
            num_leds,
            start_t,
        }
    }

    fn run(&mut self, _dt: Option<Duration>) -> Vec<Led> {
        let t = (Instant::now().duration_since(self.start_t).as_millis() as f32) / 1000.0;

        let mut frame_data: Vec<Led> = vec![Led::default(); self.num_leds];

        for (i, coordinate) in self.layout.coordinates.iter().enumerate() {
            let x = coordinate.x;
            let y = coordinate.y;

            // horizontal sinusoid
            let sine1 = f32::sin(x * 10.0 + t * 2.0);

            // rotating sinusoid
            let sine2 = f32::sin(10.0 * (x * f32::sin(t / 2.0) + y * f32::cos(t / 3.0)) + t);

            // circular sinusoid
            let cx = x + 0.5 * f32::sin(t / 5.0);
            let cy = y + 0.5 * f32::cos(t / 3.0);
            let sine3 = f32::sin(f32::sqrt(100.0 * (cx * cx + cy * cy) + 1.0) + t);

            let blend = sine1 + sine3;
            // let blend = sine3;
            // let blend = blend * (4.0 + f32::sin(t / 4.0) * 2.0);
            // let blend = blend * (2.0 + f32::sin(t / 4.0) * 2.0);

            // constrain to [0, 1]
            let blend = f32::sin(blend * PI / 2.0) / 2.0 + 0.5;

            let r = f32::abs(sine2);
            let g = 1.0 - blend;
            let b = blend * 0.5;
            // leds[i] = color.mix(color('red'), color('white'), 100 * blend);

            // if f32::abs((coordinate.x + coordinate.y) / 2.0 - t) < 0.03 {
            frame_data[i as usize].r = r;
            frame_data[i as usize].g = g;
            frame_data[i as usize].b = b;
            // }
        }

        frame_data
    }
}
