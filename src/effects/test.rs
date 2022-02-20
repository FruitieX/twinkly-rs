use std::time::{Duration, Instant};

use crate::api::DeviceLayout;

use super::{Effect, Led};

pub struct Test {
    layout: DeviceLayout,
    num_leds: usize,
    start_t: Instant,
}

impl Effect for Test {
    fn init(layout: DeviceLayout, num_leds: usize, start_t: Instant) -> Self {
        Self {
            layout,
            num_leds,
            start_t,
        }
    }

    fn run(&mut self, _dt: Option<Duration>) -> Vec<Led> {
        let t =
            ((Instant::now().duration_since(self.start_t).as_millis() as f32) % 1000.0) / 1000.0;

        let mut frame_data: Vec<Led> = vec![Led::default(); self.num_leds];

        for (i, coordinate) in self.layout.coordinates.iter().enumerate() {
            let x = coordinate.x;

            if f32::abs(x - t) < 0.03 {
                frame_data[(i) as usize].r = 1.0;
                frame_data[(i) as usize].g = 1.0;
                frame_data[(i) as usize].b = 1.0;
            }
        }

        frame_data
    }
}
