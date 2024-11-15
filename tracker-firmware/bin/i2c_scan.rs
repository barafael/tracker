#![no_std]
#![no_main]

use defmt::println;
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    i2c::{self, InterruptHandler as I2cInterruptHandler},
    peripherals::I2C0,
};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    I2C0_IRQ => I2cInterruptHandler<I2C0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let p = embassy_rp::init(Default::default());

    let sda = p.PIN_20;
    let scl = p.PIN_21;

    let mut i2c = i2c::I2c::new_async(p.I2C0, scl, sda, Irqs, i2c::Config::default());

    for addr in 1..=127 {
        println!("Scanning Address {}", addr as u8);

        // Scan Address
        let res = i2c.blocking_read(addr as u8, &mut [0]);

        // Check and Print Result
        if let Ok(()) = res {
            println!("Device Found at Address {}", addr as u8)
        }
    }
    panic!();
}
