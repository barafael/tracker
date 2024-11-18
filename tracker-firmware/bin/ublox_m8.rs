#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts, config,
    peripherals::UART0,
    uart::{self, BufferedInterruptHandler, BufferedUart},
};
use embassy_time::{Duration, Ticker};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    UART0_IRQ => BufferedInterruptHandler<UART0>;
});

const LOOP_DURATION: Duration = Duration::from_millis(10);

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let p = embassy_rp::init(config::Config::default());

    let mut ticker = Ticker::every(LOOP_DURATION);

    static TX_BUF: StaticCell<[u8; 32]> = StaticCell::new();
    let tx_buf = &mut TX_BUF.init([0; 32])[..];
    static RX_BUF: StaticCell<[u8; 32]> = StaticCell::new();
    let rx_buf = &mut RX_BUF.init([0; 32])[..];

    let config = uart::Config::default();
    let uart = BufferedUart::new(p.UART0, Irqs, p.PIN_0, p.PIN_1, tx_buf, rx_buf, config);

    let mut driver = ublox_core::new_serial_driver(uart);

    loop {
        match driver.handle_one_message() {
            Ok(msg_count) => {
                if msg_count > 0 {
                    if let Some(nav_pvt) = driver.take_last_nav_pvt() {
                        defmt::println!("NavPosVelTime: {:?}", nav_pvt);
                    }
                    if let Some(nav_dop) = driver.take_last_nav_dop() {
                        defmt::println!("NavDop: {:?}", nav_dop);
                    }
                    if let Some(mon_hw) = driver.take_last_mon_hw() {
                        defmt::println!("MonHardware: {:?}", mon_hw);
                    }
                }
            }
            Err(error) => {
                defmt::warn!("{}", error);
            }
        }
        ticker.next().await;
    }
}
