use std::time::{Duration, Instant};

use crate::api::{Coordinates, DeviceLayout};

use super::{Effect, Led};

pub struct Mix {
    frame_data: Vec<Led>,
    layout: DeviceLayout,
    start_t: Instant,
}

impl Mix {
    fn fade_out(&mut self, dt: Duration) {
        let dt = dt.as_secs_f32() * 0.5;

        for led in &mut self.frame_data {
            if led.r > 0.0 {
                let new = f32::max(0.0, led.r - dt);
                led.r = new;
            }

            if led.b > 0.0 {
                let new = f32::max(0.0, led.b - dt);
                led.b = new;
            }
        }
    }
}

impl Effect for Mix {
    fn init(layout: DeviceLayout, num_leds: usize, start_t: Instant) -> Self {
        Self {
            frame_data: vec![Led::default(); num_leds],
            layout,
            start_t,
        }
    }

    fn run(&mut self, dt: Option<Duration>) -> Vec<Led> {
        if let Some(dt) = dt {
            self.fade_out(dt);
        }

        let t =
            ((Instant::now().duration_since(self.start_t).as_millis() as f32) % 5000.0) / 5000.0;

        for (i, Coordinates { x, y, .. }) in self.layout.coordinates.iter().enumerate() {
            let y = x + y;
            let y = y / 2.0;

            if f32::abs((y - 0.1) / 2.0 - t) < 0.03 {
                self.frame_data[i as usize].r = 1.0;
            }

            if f32::abs((1.0 - y + 0.1) / 2.0 - t) < 0.03 {
                self.frame_data[i as usize].b = 1.0;
            }
        }

        self.frame_data.clone()
    }
}
