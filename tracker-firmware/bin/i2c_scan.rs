#![no_std]
#![no_main]

use defmt::{info, println, trace, Format};
use embassy_executor::Spawner;
use embassy_rp::{self, bind_interrupts, i2c, peripherals::I2C0};
use embassy_time::{with_timeout, Duration, Ticker, TimeoutError};
use embedded_hal_async::i2c::I2c;
use heapless::Vec;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    I2C0_IRQ => i2c::InterruptHandler<I2C0>;
});

const LOOP_DURATION: Duration = Duration::from_millis(1000);

#[derive(Debug, Format, thiserror::Error)]
pub enum Error {
    #[error("I2C operation timed out")]
    Timeout,

    #[error("I2C operation failed")]
    Transaction(i2c::Error),
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let config = embassy_rp::config::Config::default();
    let p = embassy_rp::init(config);

    let sda = p.PIN_20;
    let scl = p.PIN_21;

    let config = i2c::Config::default();
    let mut i2c = i2c::I2c::new_async(p.I2C0, scl, sda, Irqs, config);

    let mut ticker = Ticker::every(LOOP_DURATION);
    let mut devices: Vec<u8, 127> = heapless::Vec::new();
    loop {
        let mut changed = false;
        for addr in 1..=127 {
            trace!("Scanning Address {}", addr as u8);

            let result = with_timeout(Duration::from_millis(10), i2c.read(addr, &mut [0])).await;
            let result = match result {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => Err(Error::Transaction(e)),
                Err(TimeoutError) => Err(Error::Timeout),
            };

            let present = result.is_ok();
            let known = devices.contains(&addr);
            match (known, present) {
                (true, true) => trace!("    0x{:x} known device, unchanged", addr),
                (false, false) => trace!("No device on 0x{:x}", addr),
                (true, false) => {
                    devices.retain(|elem| *elem != addr);
                    changed = true;
                    info!("    Device at address 0x{:x} disconnected", addr);
                }
                (false, true) => {
                    devices.push(addr).unwrap();
                    changed = true;
                    info!("    Device at address 0x{:x} found", addr as u8);
                }
            }
        }
        if !devices.is_empty() && changed {
            println!("Devices: {=[u8]:#x}", devices);
        }
        ticker.next().await;
    }
}
