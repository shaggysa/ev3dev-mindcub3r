use crate::color_resolver::ColorRef;
pub(crate) use crate::distance_sensor::DistanceSensor;
use ev3dev_rs::{
    pupdevices::{ColorSensor, Motor},
    tools::wait,
    Ev3Result,
};
use std::io::BufRead;
use std::{cell::Cell, time::Duration};

pub const FLIP_SPEED: i32 = 400;
pub const COLOR_WAIT_TIME: Duration = Duration::from_millis(500);

pub const COLOR_POSITIONS: [i32; 9] = [690, 525, 590, 535, 590, 530, 590, 535, 585];

#[derive(Copy, Clone)]
pub struct CalibrationData {
    pub colors: [ColorRef; 6],
}

impl CalibrationData {
    pub fn new() -> Option<Self> {
        if let Ok(file) = std::fs::File::open(".ev3dev-mindcub3r-calibration") {
            let reader = std::io::BufReader::new(file);

            let mut arr = [ColorRef { r: 0, g: 0, b: 0 }; 6];

            for line in reader.lines() {
                let line = line.expect("failed to read line from calibration file");
                let like = line.trim();
                if like.is_empty() {
                    continue;
                }
                // Parse format: "color: r, g, b"
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() != 2 {
                    continue;
                }

                let color_name = parts[0].trim();
                let rgb_parts: Vec<&str> = parts[1].split(',').collect();
                if rgb_parts.len() != 3 {
                    continue;
                }

                let r: u16 = rgb_parts[0].trim().parse().unwrap();
                let g: u16 = rgb_parts[1].trim().parse().unwrap();
                let b: u16 = rgb_parts[2].trim().parse().unwrap();

                match color_name {
                    "white" => arr[0] = ColorRef { r, g, b },
                    "blue" => arr[1] = ColorRef { r, g, b },
                    "yellow" => arr[2] = ColorRef { r, g, b },
                    "green" => arr[3] = ColorRef { r, g, b },
                    "orange" => arr[4] = ColorRef { r, g, b },
                    "red" => arr[5] = ColorRef { r, g, b },
                    _ => continue,
                }
            }

            Some(Self { colors: arr })
        } else {
            None
        }
    }
}

pub struct Mindcub3r {
    flipper_motor: Motor,
    platform_motor: Motor,
    pub color_motor: Motor,
    pub color_sensor: ColorSensor,
    distance_sensor: DistanceSensor,
    platform_position: Cell<i32>,
    pub calibration_data: Option<CalibrationData>,
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
            calibration_data: CalibrationData::new(),
        })
    }

    pub async fn wait_for_cube(&self) -> Ev3Result<()> {
        while !self.distance_sensor.cube_present().await? {
            wait(Duration::from_millis(75)).await;
        }
        // wait an additional 1.5 seconds to allow
        // the user to move out of the way
        wait(Duration::from_millis(1500)).await;
        Ok(())
    }

    pub async fn position_flipper(&self) -> Ev3Result<()> {
        self.flipper_motor.run_target(FLIP_SPEED, 140).await?;
        self.reset_flipper().await
    }

    pub async fn flip_and_reset(&self) -> Ev3Result<()> {
        self.flipper_motor.run_target(FLIP_SPEED, 220).await?;
        self.reset_flipper().await
    }

    pub async fn reset_flipper(&self) -> Ev3Result<()> {
        self.flipper_motor.run_target(FLIP_SPEED, 65).await?;
        self.flipper_motor.run_target(150, 20).await
    }

    pub async fn reset_color_motor(&self) -> Ev3Result<()> {
        self.color_motor.run_target(1000, 420).await
    }

    pub async fn flip_and_hold(&self) -> Ev3Result<()> {
        self.flipper_motor.run_target(FLIP_SPEED, 220).await?;
        self.flipper_motor.run_target(FLIP_SPEED, 110).await
    }

    pub async fn hold_cube(&self) -> Ev3Result<()> {
        self.flipper_motor.run_target(FLIP_SPEED, 90).await
    }

    /// twist the platform by the given angle
    ///
    /// this accounts for the gear ratio between
    /// the motor and the platform
    pub async fn twist_cube(&self, angle: i32) -> Ev3Result<()> {
        // the gear ratio between the motor and the platform is 3:1
        self.platform_position.update(|pos| pos + angle * 3);

        self.platform_motor
            .run_target(500, self.platform_position.get())
            .await
    }
}
