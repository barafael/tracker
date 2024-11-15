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

const LOOP_DURATION: Duration = Duration::from_millis(10);

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let p = embassy_rp::init(Default::default());

    let sda = p.PIN_20;
    let scl = p.PIN_21;

    let i2c = i2c::I2c::new_async(p.I2C0, scl, sda, Irqs, i2c::Config::default());

    let interface = bno080::interface::I2cInterface::new(i2c, ALTERNATE_ADDRESS);

    let mut bno = BNO080::new_with_interface(interface);
    bno.init(&mut Delay).unwrap();

    bno.enable_rotation_vector(LOOP_DURATION.as_millis() as u16)
        .unwrap();

    let mut ticker = Ticker::every(LOOP_DURATION);

    loop {
        bno.handle_all_messages(&mut Delay, 1u8);
        let quat = bno.rotation_quaternion().unwrap();
        defmt::trace!("{:?}", quat);
        let quat = Q32::new(quat[0], quat[1], quat[2], quat[3]);
        let Some(quat) = quat.normalize() else {
            defmt::warn!("not normalizable");
            continue;
        };
        let euler = quat.to_euler_angles();
        defmt::info!("{:?}", (euler.roll, euler.pitch, euler.yaw));
        ticker.next().await;
    }
}
