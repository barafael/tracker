#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::pio_programs::ws2812::{PioWs2812, PioWs2812Program};
use embassy_time::{Duration, Ticker};
use itertools::Itertools;
use smart_leds::colors::BLACK;
use smart_leds::RGB8;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

const NUM_LEDS: usize = 57;
const COLOR: RGB8 = RGB8::new(210, 30, 10);

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Start");
    let p = embassy_rp::init(Default::default());

    let Pio {
        mut common, sm0, ..
    } = Pio::new(p.PIO0, Irqs);

    let mut data = [RGB8::default(); NUM_LEDS];

    let program = PioWs2812Program::new(&mut common);
    let mut ws2812 = PioWs2812::new(&mut common, sm0, p.DMA_CH0, p.PIN_16, &program);

    let cycle = (0..NUM_LEDS).cycle().skip(NUM_LEDS - 1).tuple_windows();

    let mut ticker = Ticker::every(Duration::from_millis(300));
    for (last, current) in cycle {
        data[last] = BLACK;
        data[current] = COLOR;
        ws2812.write(&data).await;

        ticker.next().await;
    }
}
