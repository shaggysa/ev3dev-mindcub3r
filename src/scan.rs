use crate::color_resolver::Rgb;
use crate::mindcub3r::{Mindcub3r, COLOR_POSITIONS, COLOR_WAIT_TIME};
use ev3dev_rs::tools::wait;
use ev3dev_rs::{join, Ev3Result};
use phf::{phf_map, Map};

static LFRB_MAP: Map<usize, usize> = phf_map! {
    0 => 4,
    1 => 6,
    2 => 7,
    3 => 8,
    4 => 5,
    5 => 2,
    6 => 1,
    7 => 0,
    8 => 3
};

/*
   CUBE LAYOUT
       U U U
       U U U
       U U U
L L L  F F F  R R R  B B B
L L L  F F F  R R R  B B B
L L L  F F F  R R R  B B B
       D D D
       D D D
       D D D
   */

impl Mindcub3r {
    pub async fn scan_side(&self, map: &Map<usize, usize>) -> Ev3Result<[Rgb; 9]> {
        self.color_motor
            .run_target(1000, COLOR_POSITIONS[0])
            .await?;
        // allow time for the color sensor to update
        wait(COLOR_WAIT_TIME).await;

        let mut arr = [Rgb { r: 0, g: 0, b: 0 }; 9];

        // center square
        let (r, g, b) = self.color_sensor.raw_rgb().await?;
        arr[*map.get(&0).unwrap()] = Rgb { r, g, b };

        for i in 1..9 {
            join!(
                self.color_motor.run_target(1000, COLOR_POSITIONS[i]),
                self.twist_cube(45)
            )?;

            // allow time for the color sensor to update
            wait(COLOR_WAIT_TIME).await;
            let (r, g, b) = self.color_sensor.raw_rgb().await?;
            arr[*map.get(&i).unwrap()] = Rgb { r, g, b };
        }
        Ok(arr)
    }

    pub async fn scan_all_temp(&self) -> Ev3Result<[Rgb; 54]> {
        let mut white = true;

        let mut colors = [Rgb { r: 0, g: 0, b: 0 }; 54];

        self.position_flipper().await?;

        for i in 0..4 {
            if !white {
                self.reset_color_motor().await?;
                self.flip_and_reset().await?;
            }

            for (idx, color) in self.scan_lfrb().await?.iter().enumerate() {
                colors[(i * 9) + idx] = *color;
            }

            white = false;
        }

        self.reset_color_motor().await?;
        self.twist_cube(90).await?;
        self.flip_and_reset().await?;
        for (idx, color) in self.scan_lfrb().await?.iter().enumerate() {
            colors[(4 * 9) + idx] = *color;
        }

        self.reset_color_motor().await?;
        self.flip_and_hold().await?;
        self.flip_and_reset().await?;
        for (idx, color) in self.scan_lfrb().await?.iter().enumerate() {
            colors[(5 * 9) + idx] = *color;
        }

        Ok(colors)
    }

    pub async fn scan_lfrb(&self) -> Ev3Result<[Rgb; 9]> {
        self.scan_side(&LFRB_MAP).await
    }
}
