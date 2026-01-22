use std::time::Duration;

use ev3dev_rs::parameters::{Direction, MotorPort, SensorPort, Stop};
use ev3dev_rs::pupdevices::{ColorSensor, InfraredSensor, Motor, UltrasonicSensor};
use ev3dev_rs::tools::wait;
use ev3dev_rs::{Ev3Error, Ev3Result};

enum DistanceSensor {
    Ultrasonic(UltrasonicSensor),
    Infrared(InfraredSensor),
}

impl DistanceSensor {
    fn new(port: SensorPort) -> Ev3Result<Self> {
        if let Ok(ultrasonic_sensor) = UltrasonicSensor::new(port) {
            Ok(DistanceSensor::Ultrasonic(ultrasonic_sensor))
        } else if let Ok(infrared_sensor) = InfraredSensor::new(port) {
            Ok(DistanceSensor::Infrared(infrared_sensor))
        } else {
            Err(Ev3Error::NoSensorProvided)
        }
    }

    fn cube_present(&self) -> Ev3Result<bool> {
        match self {
            DistanceSensor::Ultrasonic(sensor) => Ok(sensor.distance_cm()? < 50.0),

            DistanceSensor::Infrared(sensor) => Ok(sensor.proximity()? < 10),
        }
    }
}

#[tokio::main]
async fn main() -> Ev3Result<()> {
    let distance_sensor = DistanceSensor::new(SensorPort::In1)?;
    let color_sensor = ColorSensor::new(SensorPort::In2)?;

    let flipper_motor = Motor::new(MotorPort::OutA, Direction::Clockwise)?;
    let platform_motor = Motor::new(MotorPort::OutB, Direction::CounterClockwise)?;
    let color_motor = Motor::new(MotorPort::OutC, Direction::CounterClockwise)?;

    flipper_motor.reset()?;
    color_motor.reset()?;
    platform_motor.reset()?;

    flipper_motor.set_stop_action(Stop::Hold)?;
    color_motor.set_stop_action(Stop::Hold)?;
    platform_motor.set_stop_action(Stop::Hold)?;

    // this must be run first because the color sensor could block the flipper motor
    color_motor.run_until_stalled(-75).await?;

    flipper_motor.run_until_stalled(-75).await?;

    while !distance_sensor.cube_present()? {
        wait(Duration::from_millis(100)).await;
    }

    // wait for the user to move their hand out of the way
    wait(Duration::from_secs(3)).await;

    loop {
        wait(Duration::from_millis(250)).await;
        platform_motor.run_target(1000, 270).await?;
        wait(Duration::from_millis(250)).await;
        platform_motor.run_target(1000, 0).await?;
    }
}
