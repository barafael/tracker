#![no_std]
#![no_main]

use bno080::{interface::i2c::ALTERNATE_ADDRESS, wrapper::BNO080};
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    i2c::{self, InterruptHandler as I2cInterruptHandler},
    peripherals::I2C0,
};
use embassy_time::{Delay, Duration, Ticker};
use num_quaternion::Q32;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    I2C0_IRQ => I2cInterruptHandler<I2C0>;
});

const BNO_UPDATE_PERIOD: Duration = Duration::from_millis(10);

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let config = embassy_rp::config::Config::default();
    let p = embassy_rp::init(config);

    // IMU driver.
    let sda = p.PIN_20;
    let scl = p.PIN_21;

    let config = i2c::Config::default();
    let i2c = i2c::I2c::new_async(p.I2C0, scl, sda, Irqs, config);

    let interface = bno080::interface::I2cInterface::new(i2c, ALTERNATE_ADDRESS);

    let mut imu = BNO080::new_with_interface(interface);
    imu.init(&mut Delay).expect("Failed to initialize BNO");
    imu.enable_rotation_vector(BNO_UPDATE_PERIOD.as_millis() as u16)
        .expect("Failed to enable rotation vector on BNO");

    let mut ticker = Ticker::every(BNO_UPDATE_PERIOD);
    loop {
        imu.handle_all_messages(&mut Delay, 1);
        let quaternion = imu.rotation_quaternion().unwrap();
        defmt::trace!("{:?}", quaternion);
        let quaternion = Q32::new(quaternion[0], quaternion[1], quaternion[2], quaternion[3]);
        let Some(quaternion) = quaternion.normalize() else {
            defmt::warn!("not normalizable");
            continue;
        };
        let angles = quaternion.to_euler_angles();
        defmt::info!("{:?}", (angles.roll, angles.pitch, angles.yaw));

        ticker.next().await;
    }
}
