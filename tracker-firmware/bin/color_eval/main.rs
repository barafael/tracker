#![no_std]
#![no_main]

use color_wheel::{COLORS, COLOR_NAMES};
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    peripherals::PIO0,
    pio::{InterruptHandler as PioInterruptHandler, Pio},
    pio_programs::ws2812::{PioWs2812, PioWs2812Program},
};
use embassy_time::{Duration, Ticker};
use smart_leds::colors::BLACK;
use smart_leds::RGB8;
use {defmt_rtt as _, panic_probe as _};

mod color_wheel;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
});

const NUM_LEDS: usize = 10;
const LOOP_DURATION: Duration = Duration::from_millis(80);

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let p = embassy_rp::init(Default::default());

    let Pio {
        mut common, sm0, ..
    } = Pio::new(p.PIO0, Irqs);

    let ws2812_program = PioWs2812Program::new(&mut common);
    let mut led_strip = PioWs2812::new(&mut common, sm0, p.DMA_CH0, p.PIN_16, &ws2812_program);

    let mut leds = [RGB8::default(); NUM_LEDS];

    let mut ticker = Ticker::every(LOOP_DURATION);

    for (color_index, i) in (0..57).cycle().zip((0..COLORS.len()).cycle()) {
        let color = COLORS[color_index % COLORS.len()];
        let name = COLOR_NAMES[color_index % COLORS.len()];
        println!("{}", name);
        leds.iter_mut().for_each(|l| *l = BLACK);
        leds[i] = color;

        led_strip.write(&leds).await;

        ticker.next().await;
    }
    defmt::panic!();
}
