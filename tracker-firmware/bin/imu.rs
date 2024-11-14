#![no_std]
#![no_main]

use bno080::{interface::i2c::DEFAULT_ADDRESS, wrapper::BNO080};
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    i2c::{self, InterruptHandler as I2cInterruptHandler},
    peripherals::I2C1,
};
use embassy_time::{Delay, Duration, Ticker};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    I2C1_IRQ => I2cInterruptHandler<I2C1>;
});

const LOOP_DURATION: Duration = Duration::from_millis(50);

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let p = embassy_rp::init(Default::default());

    let sda = p.PIN_14;
    let scl = p.PIN_15;

    let i2c = i2c::I2c::new_async(p.I2C1, scl, sda, Irqs, i2c::Config::default());

    let interface = bno080::interface::I2cInterface::new(i2c, DEFAULT_ADDRESS);

    let mut bno = BNO080::new_with_interface(interface);
    bno.init(&mut Delay).unwrap();

    bno.enable_rotation_vector(LOOP_DURATION.as_millis() as u16)
        .unwrap();

    let mut ticker = Ticker::every(LOOP_DURATION);

    loop {
        bno.handle_all_messages(&mut Delay, 1u8);
        let quat = bno.rotation_quaternion().unwrap();
        defmt::println!("{}", quat);

        ticker.next().await;
    }
}
