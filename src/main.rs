mod mindcub3r;

use ev3dev_rs::parameters::{Direction, MotorPort, SensorPort};
use ev3dev_rs::pupdevices::{ColorSensor, Motor};
use ev3dev_rs::Ev3Result;
use mindcub3r::{DistanceSensor, Mindcub3r};

#[tokio::main]
async fn main() -> Ev3Result<()> {
    let distance_sensor = DistanceSensor::new(SensorPort::In1)?;
    let color_sensor = ColorSensor::new(SensorPort::In2)?;

    let flipper_motor = Motor::new(MotorPort::OutA, Direction::Clockwise)?;
    let platform_motor = Motor::new(MotorPort::OutB, Direction::Clockwise)?;
    let color_motor = Motor::new(MotorPort::OutC, Direction::CounterClockwise)?;

    let mindcub3r = Mindcub3r::new(
        flipper_motor,
        platform_motor,
        color_motor,
        color_sensor,
        distance_sensor,
    )
    .await?;

    mindcub3r.wait_for_cube().await?;
    mindcub3r.scan_side().await?;
    mindcub3r.scan_side().await?;

    Ok(())
}
