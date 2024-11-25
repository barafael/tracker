#![cfg_attr(not(test), no_std)]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    peripherals::PIO0,
    pio::{InterruptHandler as PioInterruptHandler, Pio},
    pio_programs::ws2812::{PioWs2812, PioWs2812Program},
};
use embassy_time::{Duration, Ticker};
use smart_leds::{colors::BLACK, RGB8};
use {defmt_rtt as _, panic_probe as _};

use color_wheel::{COLORS, COLOR_NAMES};
use tracker_firmware::adjust_color_for_led_type;

mod color_wheel;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
});

const NUM_LEDS: usize = 57;
const LOOP_DURATION: Duration = Duration::from_millis(300);

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let config = embassy_rp::config::Config::default();
    let p = embassy_rp::init(config);

    let Pio {
        mut common, sm0, ..
    } = Pio::new(p.PIO0, Irqs);

    let ws2812_program = PioWs2812Program::new(&mut common);
    let mut led_strip = PioWs2812::new(&mut common, sm0, p.DMA_CH0, p.PIN_16, &ws2812_program);

    let mut leds = [RGB8::default(); NUM_LEDS];

    let mut ticker = Ticker::every(LOOP_DURATION);

    let leds_cycled = (0..NUM_LEDS).cycle();
    let colors_cycled = (0..COLORS.len()).cycle();
    for (led_index, color_index) in leds_cycled.zip(colors_cycled) {
        let color = adjust_color_for_led_type(COLORS[color_index]);
        let name = COLOR_NAMES[color_index];
        defmt::println!("{}", name);

        leds.iter_mut().for_each(|l| *l = BLACK);
        leds[led_index] = color;

        led_strip.write(&leds).await;

        ticker.next().await;
    }
    defmt::unreachable!("infinite loop wasn't infinite. Rethink your life choices.");
}
