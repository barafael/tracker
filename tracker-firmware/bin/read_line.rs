#![no_std]
#![no_main]

use core::str::from_utf8;

use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts, config,
    peripherals::UART0,
    uart::{self, BufferedInterruptHandler, BufferedUart},
};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    UART0_IRQ => BufferedInterruptHandler<UART0>;
});

const BUFFER_SIZE: usize = 256;

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let p = embassy_rp::init(config::Config::default());

    static TX_BUF: StaticCell<[u8; BUFFER_SIZE]> = StaticCell::new();
    let tx_buf = &mut TX_BUF.init([0; BUFFER_SIZE])[..];
    static RX_BUF: StaticCell<[u8; BUFFER_SIZE]> = StaticCell::new();
    let rx_buf = &mut RX_BUF.init([0; BUFFER_SIZE])[..];

    let mut config = uart::Config::default();
    config.baudrate = 9600;
    let uart = BufferedUart::new(p.UART0, Irqs, p.PIN_0, p.PIN_1, tx_buf, rx_buf, config);

    let mut reader = lines_codec::ReadLine::<_, BUFFER_SIZE>::new(uart);

    let mut line = [0u8; BUFFER_SIZE];
    loop {
        let Ok(len) = reader
            .read_line_async(&mut line)
            .await
            .inspect_err(|e| defmt::warn!("{}", e))
        else {
            continue;
        };
        defmt::println!("{}", from_utf8(&line[..len]).ok());
    }
}
