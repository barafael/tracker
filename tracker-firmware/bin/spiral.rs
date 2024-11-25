#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    peripherals::PIO0,
    pio::{InterruptHandler, Pio},
    pio_programs::ws2812::{PioWs2812, PioWs2812Program},
};
use embassy_time::{Duration, Ticker};
use smart_leds::{colors, RGB8};
use {defmt_rtt as _, panic_probe as _};

use tracker_firmware::adjust_color_for_led_type;
use tracker_mapper::Coordinate;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

const NUM_LEDS: usize = 57;
const COLOR: RGB8 = colors::ORANGE_RED;
const LOOP_DURATION: Duration = Duration::from_millis(10);

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

    let mut color = adjust_color_for_led_type(COLOR);
    color.g -= 40; // make it nice orange color.

    let mut previous_led_index = 0;
    let mut ticker = Ticker::every(LOOP_DURATION);
    loop {
        for distance in 0..5 {
            for angle in 0..16 {
                let coordinate = Coordinate::from_world_coordinates(distance, angle * (360 / 16));
                let led_index = tracker_mapper::index_of(coordinate) as usize;

                leds[previous_led_index] = colors::BLACK;
                previous_led_index = led_index;
                leds[led_index] = color;

                led_strip.write(&leds).await;

                ticker.next().await;
            }
        }
    }
}
