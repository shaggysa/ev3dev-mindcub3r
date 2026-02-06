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

#[derive(Debug, Copy, Clone)]
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

    async fn get_square_color(&self) -> Ev3Result<CubeColor> {
        let (mut r, mut g, mut b) = (0, 0, 0);
        let found: CubeColor;

        for _ in 0..5 {
            let (r_curr, g_curr, b_curr) = self.color_sensor.raw_rgb()?;
            r += r_curr;
            g += g_curr;
            b += b_curr;
            wait(Duration::from_millis(5)).await;
        }

        r /= 5;
        g /= 5;
        b /= 5;

        let max_val = r.max(g).max(b);
        let min_val = r.min(g).min(b);

        // Check for white (all channels high and balanced)
        // White: (179, 179, 253), (256, 249, 327)
        if min_val > 150 && max_val > 200 {
            found = CubeColor::White;
        }
        // Check for yellow (high red and green, lower blue)
        // Yellow: (177, 166, 85), (186, 165, 88)
        else if r > 150 && g > 140 && b < 120 {
            found = CubeColor::Yellow;
        }
        // Check for orange (red dominant, high values, g close to b)
        // Orange: (237, 95, 81)
        else if r > 200 && g > 70 && g < 150 && b < 120 {
            found = CubeColor::Orange;
        }
        // Check for blue (blue is clearly dominant and high)
        // Blue: (41, 42, 117)
        else if b == max_val && b > 100 {
            found = CubeColor::Blue;
        }
        // Check for green (green is clearly dominant)
        // Green: (57, 133, 93), (53, 126, 78)
        else if g == max_val && g > r && g > b {
            found = CubeColor::Green;
        }
        // Check for red (red dominant, lower values OR g > b significantly)
        // Red: (84, 42, 44), (144, 41, 68)
        else if r == max_val && r > 70 {
            found = CubeColor::Red;
        }
        // Fallback
        else {
            found = CubeColor::White;
        }

        println!("r: {}, g: {}, b: {}, detected: {:?}", r, g, b, found);

        Ok(found)
    }

    pub async fn flip_and_reset(&self) -> Ev3Result<()> {
        self.flipper_motor.run_target(500, 195).await?;
        self.flipper_motor.run_target(500, 0).await
    }

    pub async fn reset_flipper(&self) -> Ev3Result<()> {
        self.flipper_motor.run_target(1000, 0).await
    }

    pub async fn reset_color_motor(&self) -> Ev3Result<()> {
        self.color_motor.run_target(1000, 420).await
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

    pub async fn scan_side(&self) -> Ev3Result<[CubeColor; 9]> {
        self.color_motor.run_target(1000, 765).await?;

        let mut arr = [CubeColor::White; 9];
        arr[0] = self.get_square_color().await?;

        for i in (1..9).step_by(2) {
            join!(self.color_motor.run_target(1000, 900), self.twist_cube(45))?;
            arr[i] = self.get_square_color().await?;

            join!(self.color_motor.run_target(1000, 830), self.twist_cube(45))?;
            arr[i + 1] = self.get_square_color().await?;
        }

        Ok(arr)
    }
}
