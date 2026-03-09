use crate::mindcub3r::{CalibrationData, Mindcub3r, COLOR_POSITIONS, COLOR_WAIT_TIME};
use ev3dev_rs::tools::wait;
use ev3dev_rs::{join, Ev3Result};
use std::fs::File;
use std::io::{BufWriter, Write};

impl Mindcub3r {
    /// returns avg: (r, g, b)
    async fn get_side_avg(&self) -> Ev3Result<(u16, u16, u16)> {
        self.color_motor
            .run_target(1000, COLOR_POSITIONS[0])
            .await?;

        wait(COLOR_WAIT_TIME).await;

        let mut r_sum = 0;
        let mut g_sum = 0;
        let mut b_sum = 0;

        let (mut r, mut g, mut b) = self.color_sensor.raw_rgb().await?;

        r_sum += r as i32;
        g_sum += g as i32;
        b_sum += b as i32;

        for i in 1..9 {
            join!(
                self.color_motor.run_target(1000, COLOR_POSITIONS[i]),
                self.twist_cube(45)
            )?;

            wait(COLOR_WAIT_TIME).await;

            (r, g, b) = self.color_sensor.raw_rgb().await?;
            r_sum += r as i32;
            g_sum += g as i32;
            b_sum += b as i32;
        }

        Ok(((r_sum / 9) as u16, (g_sum / 9) as u16, (b_sum / 9) as u16))
    }

    pub async fn calibrate(&mut self) -> Ev3Result<()> {
        let file = File::create(".ev3dev-mindcub3r-calibration")
            .expect("failed to create calibration file");

        let mut writer = BufWriter::new(file);

        let colors = ["white", "blue", "yellow", "green"];

        self.position_flipper().await?;

        for color in colors {
            if color != "white" {
                self.reset_color_motor().await?;
                self.flip_and_reset().await?;
            }
            let (r, g, b) = self.get_side_avg().await?;
            writeln!(writer, "{color}: {r}, {g}, {b}").expect("failed to write calibration data");
        }

        self.reset_color_motor().await?;
        self.twist_cube(90).await?;
        self.flip_and_reset().await?;
        let (r, g, b) = self.get_side_avg().await?;
        writeln!(writer, "orange: {r}, {g}, {b}").expect("failed to write calibration data");

        self.reset_color_motor().await?;
        self.flip_and_hold().await?;
        self.flip_and_reset().await?;
        let (r, g, b) = self.get_side_avg().await?;
        writeln!(writer, "red: {r}, {g}, {b}").expect("failed to write calibration data");

        writer.flush().expect("failed to flush calibration data");

        self.calibration_data = CalibrationData::new();

        Ok(())
    }
}
