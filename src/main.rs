mod calibrate;
mod color_resolver;
mod distance_sensor;
mod mindcub3r;
mod scan;

use crate::color_resolver::assign_colors;
use crate::mindcub3r::{DistanceSensor, Mindcub3r};
use ev3dev_rs::parameters::{Direction, MotorPort, SensorPort};
use ev3dev_rs::pupdevices::{ColorSensor, Motor};
use ev3dev_rs::Ev3Result;

#[tokio::main]
async fn main() -> Ev3Result<()> {
    let distance_sensor = DistanceSensor::new(SensorPort::In3)?;
    let color_sensor = ColorSensor::new(SensorPort::In1)?;

    let flipper_motor = Motor::new(MotorPort::OutA, Direction::Clockwise).await?;
    let platform_motor = Motor::new(MotorPort::OutB, Direction::Clockwise).await?;
    let color_motor = Motor::new(MotorPort::OutC, Direction::CounterClockwise).await?;

    let mut mindcub3r = Mindcub3r::new(
        flipper_motor,
        platform_motor,
        color_motor,
        color_sensor,
        distance_sensor,
    )
        .await?;

    mindcub3r.wait_for_cube().await?;

    if std::env::args().any(|arg| arg == "calibrate") {
        mindcub3r.calibrate().await?;
        return Ok(());
    }

    let refs = mindcub3r
        .calibration_data
        .expect("calibration data not found!");

    let samples = mindcub3r.scan_all_temp().await?;

    let result = assign_colors(&samples, &refs.colors);

    for (f, (&sample, &color)) in samples.iter().zip(result.iter()).enumerate() {
        println!(
            "face {:2}: assigned={} rgb=({:3},{:3},{:3})",
            f, color, sample.r, sample.g, sample.b
        );
    }

    // for i in 500..1000 {
    //     mindcub3r.color_motor.run_target(1000, i).await?;
    //     println!("{i}");
    // }

    //mindcub3r.calibrate().await?;

    use nanoserde::DeJson;
    use std::time::Duration;

    #[derive(Debug, DeJson, Clone, Copy)]
    struct SolveTime {
        secs: u64,
        nanos: u32,
    }

    impl From<SolveTime> for Duration {
        fn from(st: SolveTime) -> Self {
            Duration::new(st.secs, st.nanos)
        }
    }

    // #[derive(Debug, DeJson)]
    // struct CubeResult {
    //     solution: Vec<String>,
    //     solve_time: SolveTime,
    // }
    //
    // let res = dbg!(
    //     ureq::get(
    //         "http://192.168.0.2:3000/solve/RLLBUFUUUBDURRBBUBRLRRFDFDDLLLUDFLRRDDFRLFDBUBFFLBBDUF"
    //     )
    //     .call()
    //     .expect("request failed")
    // );
    //
    // let body = res.into_string().expect("failed to read body");
    // let cube_result: CubeResult = DeJson::deserialize_json(&body).expect("failed to parse json");
    // let solve_duration: Duration = cube_result.solve_time.into();
    // dbg!(&cube_result);
    // dbg!(solve_duration);

    Ok(())
}
