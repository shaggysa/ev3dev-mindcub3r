use std::{cell::Cell, time::Duration};

use ev3dev_rs::{
    join, parameters::SensorPort, pupdevices::{ColorSensor, InfraredSensor, Motor, UltrasonicSensor},
    tools::wait,
    Ev3Error,
    Ev3Result,
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

    async fn cube_present(&self) -> Ev3Result<bool> {
        match self {
            // take five samples over 25 ms to prevent
            // an outlier from causing a false positive
            DistanceSensor::Ultrasonic(sensor) => {
                for _ in 0..5 {
                    if dbg!(sensor.distance_cm()?) > 8.0 {
                        return Ok(false);
                    }
                    wait(Duration::from_millis(5)).await;
                }
                Ok(true)
            }

            DistanceSensor::Infrared(sensor) => {
                for _ in 0..5 {
                    if sensor.proximity()? > 5 {
                        return Ok(false);
                    }
                    wait(Duration::from_millis(5)).await;
                }
                Ok(true)
            }
        }
    }
}

#[derive(Debug)]
pub enum CubeColor {
    White,
    Red,
    Yellow,
    Orange,
    Green,
    Blue,
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
        while !self.distance_sensor.cube_present().await? {
            wait(Duration::from_millis(75)).await;
        }
        // wait an additional 2 seconds to allow
        // the user to move out of the way
        wait(Duration::from_secs(2)).await;
        Ok(())
    }

    fn get_square_color(&self) -> Ev3Result<CubeColor> {
        let (r, g, b) = self.color_sensor.raw_rgb()?;

        let max_val = r.max(g).max(b);
        let min_val = r.min(g).min(b);

        // Check for white (high values across all channels)
        // min_val > 0.6 * 1020 = 612
        if min_val > 612 {
            return Ok(CubeColor::White);
        }

        // Check for yellow (high red and green, low blue)
        // r > 510, g > 510, b < 408
        if r > 510 && g > 510 && b < 408 {
            return Ok(CubeColor::Yellow);
        }

        // Check for orange (high red, medium green, low blue)
        // r > 510, 255 < g < 612, b < 306
        if r > 510 && g > 255 && g < 612 && b < 306 {
            return Ok(CubeColor::Orange);
        }

        // Determine color based on which channel is dominant
        // r > g * 1.3 becomes r * 10 > g * 13
        if r == max_val && r * 10 > g * 13 {
            Ok(CubeColor::Red)
        } else if g == max_val && g * 10 > r * 12 {
            Ok(CubeColor::Green)
        } else if b == max_val {
            Ok(CubeColor::Blue)
        } else {
            // Fallback
            Ok(CubeColor::White)
        }
    }

    pub async fn flip_and_reset(&self) -> Ev3Result<()> {
        self.flipper_motor.run_target(500, 195).await?;
        self.flipper_motor.run_target(500, 0).await
    }

    pub async fn reset_flipper(&self) -> Ev3Result<()> {
        self.flipper_motor.run_target(1000, 0).await
    }

    pub async fn flip_and_hold(&self) -> Ev3Result<()> {
        self.flipper_motor.run_target(500, 195).await?;
        self.flipper_motor.run_target(500, 110).await
    }

    pub async fn hold_cube(&self) -> Ev3Result<()> {
        self.flipper_motor.run_target(1000, 90).await
    }

    /// twist the platform by the given angle
    ///
    /// this accounts for the gear ratio between
    /// the motor and the platform
    pub async fn twist_cube(&self, angle: i32) -> Ev3Result<()> {
        // the gear ratio between the motor and the platform is 3:1
        self.platform_position.update(|pos| pos + angle * 3);

        self.platform_motor
            .run_target(1000, self.platform_position.get())
            .await
    }

    pub async fn scan_side(&self) -> Ev3Result<()> {
        self.color_motor.run_target(1000, 765).await?;
        dbg!(self.color_sensor.raw_rgb()?);

        loop {
            join!(self.color_motor.run_target(1000, 900), self.twist_cube(45))?;
            dbg!(self.get_square_color()?);

            join!(self.color_motor.run_target(1000, 830), self.twist_cube(45))?;
            dbg!(self.get_square_color()?);
        }
    }
}
