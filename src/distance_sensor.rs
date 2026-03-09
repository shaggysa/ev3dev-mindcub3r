use ev3dev_rs::parameters::SensorPort;
use ev3dev_rs::pupdevices::{InfraredSensor, UltrasonicSensor};
use ev3dev_rs::tools::wait;
use ev3dev_rs::{Ev3Error, Ev3Result};
use std::time::Duration;

/// An enum representing an ultrasonic sensor or an infrared sensor.
/// This allows them to be used interchangeably in the Mindcub3r struct.
pub enum DistanceSensor {
    Ultrasonic(UltrasonicSensor),
    Infrared(InfraredSensor),
}

impl DistanceSensor {
    pub fn new(port: SensorPort) -> Ev3Result<Self> {
        if let Ok(ultrasonic_sensor) = UltrasonicSensor::new(port) {
            Ok(DistanceSensor::Ultrasonic(ultrasonic_sensor))
        } else if let Ok(infrared_sensor) = InfraredSensor::new(port) {
            Ok(DistanceSensor::Infrared(infrared_sensor))
        } else {
            Err(Ev3Error::NoSensorProvided)
        }
    }

    pub(crate) async fn cube_present(&self) -> Ev3Result<bool> {
        match self {
            // take five samples over 25 ms to prevent
            // an outlier from causing a false positive
            DistanceSensor::Ultrasonic(sensor) => {
                for _ in 0..5 {
                    if sensor.distance_cm().await? > 8.0 {
                        return Ok(false);
                    }
                    wait(Duration::from_millis(5)).await;
                }
                Ok(true)
            }

            DistanceSensor::Infrared(sensor) => {
                for _ in 0..5 {
                    if dbg!(sensor.proximity().await?) > 17 {
                        return Ok(false);
                    }
                    wait(Duration::from_millis(5)).await;
                }
                Ok(true)
            }
        }
    }
}
