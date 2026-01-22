use ev3dev_rs::Ev3Result;
use ev3dev_rs::parameters::{Direction, MotorPort, SensorPort};
use ev3dev_rs::pupdevices::{GyroSensor, Motor};
use ev3dev_rs::robotics::DriveBase;

#[tokio::main]
async fn main() -> Ev3Result<()> {
    let motor = Motor::new(MotorPort::OutA, Direction::CounterClockwise)?;
    let motor2 = Motor::new(MotorPort::OutD, Direction::CounterClockwise)?;
    let gyro = GyroSensor::new(SensorPort::In3)?;

    let drive = DriveBase::new(&motor, &motor2, 62.4, 155.0)?.with_gyro(&gyro)?;

    drive.use_gyro(true)?;
    drive.set_turn_speed(400);

    loop {
        drive.straight(250).await?;
        drive.turn(90).await?;
    }
}
