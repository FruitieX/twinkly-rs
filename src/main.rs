use crate::{
    api::TwinklyApi,
    effects::{Effect, *},
};
use std::time::{Duration, Instant};

mod api;
mod effects;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let addr = args.get(1).expect("Expected host as first argument.");

    let api = TwinklyApi::new(addr.clone());

    // Get device status
    let status = api.get_status().await?;

    // Get device firmware version
    let version = api.get_fw_version().await?;
    dbg!(version);

    // Get device mode
    let original_mode = api.get_mode().await?;

    // Get device layout
    let layout = api.get_layout().await?.normalized_coords();

    {
        let api = api.clone();

        tokio::spawn(async move {
            (async {
                // Open UDP socket
                api.init_udp().await?;

                let start_t = Instant::now();
                let mut effect = mix::Mix::init(layout.clone(), status.number_of_led, start_t);

                let mut prev_frame: Option<Instant> = None;

                loop {
                    let frame_data = effect.run(prev_frame.map(|t| t.elapsed()));
                    prev_frame = Some(Instant::now());

                    let frame_data = frame_data
                        .iter()
                        .map(|led| led.to_gamma_corrected_bytes())
                        .flatten()
                        .collect();

                    api.send_rt_frame(frame_data).await?;
                    let duration =
                        Duration::from_millis((1000.0 / status.measured_frame_rate) as u64);
                    tokio::time::sleep(duration).await;
                }

                // Need to annotate Result type here or this won't compile
                #[allow(unreachable_code)]
                Ok::<(), Box<dyn std::error::Error>>(())
            })
            .await
            .ok();
        });
    }

    match tokio::signal::ctrl_c().await {
        Ok(()) => {
            println!("ctrl-c received, restoring original mode");
            api.set_mode(original_mode).await?;
        }
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
        }
    }

    Ok(())
}
