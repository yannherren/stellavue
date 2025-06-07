use embedded_hal::i2c::I2c;
use esp_idf_svc::hal::delay::Ets;
use esp_idf_svc::hal::gpio::OutputPin;
use mpu6050_dmp::address::Address;
use mpu6050_dmp::error::{Error, InitError};
use mpu6050_dmp::gyro::Gyro;
use mpu6050_dmp::sensor::Mpu6050;

pub struct CameraModule<I, P>
where
    I: I2c,
    P: OutputPin
{
    mpu6050: Mpu6050<I>,
    shutter_pin: P
}

impl<I, P> CameraModule<I, P>
where
    I: I2c,
    P: OutputPin
{
    pub fn new(i2c: I, shutter_pin: P) -> Result<Self, InitError<I>> {
        Ok(CameraModule {
            mpu6050: Mpu6050::new(i2c, Address::default())?,
            shutter_pin
        })
    }

    pub fn init(&mut self) -> Result<(), Error<I>> {
        let mut delay = Ets;
        self.mpu6050.initialize_dmp(&mut delay)?;
        Ok(())
    }

    pub fn get_acc(&mut self) -> Result<Gyro, Error<I>> {
        self.mpu6050.gyro()
    }


}