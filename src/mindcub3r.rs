use std::{cell::Cell, time::Duration};

use ev3dev_rs::{
    Ev3Error, Ev3Result,
    parameters::{SensorPort, Stop},
    pupdevices::{ColorSensor, InfraredSensor, Motor, UltrasonicSensor},
    tools::wait,
};

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

    fn cube_present(&self) -> bool {
        match self {
            // the distance sensor sometimes sends invalid data, so
            // we retry until we get a valid reading
            DistanceSensor::Ultrasonic(sensor) => loop {
                if let Ok(distance) = sensor.distance_cm() {
                    return distance < 10.0;
                }
            },

            DistanceSensor::Infrared(sensor) => loop {
                if let Ok(prox) = sensor.proximity() {
                    return prox < 5;
                }
            },
        }
    }
}

pub struct Mindcub3r {
    flipper_motor: Motor,
    platform_motor: Motor,
    color_motor: Motor,
    color_sensor: ColorSensor,
    distance_sensor: DistanceSensor,
    platform_position: Cell<i32>,
}

impl Mindcub3r {
    // initialize the Mindcub3r and return an object
    pub async fn new(
        flipper_motor: Motor,
        platform_motor: Motor,
        color_motor: Motor,
        color_sensor: ColorSensor,
        distance_sensor: DistanceSensor,
    ) -> Ev3Result<Self> {
        // these can't be run simultaneously because
        // the color sensor can block the flipper arm
        color_motor.run_until_stalled(-75).await?;
        flipper_motor.run_until_stalled(-75).await?;

        // ensure that the starting position of all the motors is zero
        flipper_motor.reset()?;
        color_motor.reset()?;
        platform_motor.reset()?;

        flipper_motor.hold()?;
        platform_motor.hold()?;
        color_motor.hold()?;

        Ok(Mindcub3r {
            flipper_motor,
            platform_motor,
            color_motor,
            color_sensor,
            distance_sensor,
            platform_position: Cell::new(0),
        })
    }

    pub async fn wait_for_cube(&self) -> Ev3Result<()> {
        while !self.distance_sensor.cube_present() {
            wait(Duration::from_millis(100)).await;
        }
        // wait an additional 2 seconds to allow
        //  the user to move out of the way
        wait(Duration::from_secs(2)).await;
        Ok(())
    }

    pub async fn flip_cube(&self) -> Ev3Result<()> {
        self.flipper_motor.run_target(600, 180).await?;
        self.flipper_motor.run_target(1000, 0).await
    }

    pub async fn hold_cube(&self) -> Ev3Result<()> {
        self.flipper_motor.run_target(1000, 90).await
    }

    /// twist the platform by the given angle
    pub async fn twist_cube(&self, angle: i32) -> Ev3Result<()> {
        self.platform_position.update(|pos| pos + angle * 3);
        self.platform_motor
            .run_target(1000, self.platform_position.get())
            .await
    }
}
