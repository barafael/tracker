#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    clocks::{self, RoscRng},
    config,
    peripherals::PIO0,
    pio::{InterruptHandler, Pio},
    pio_programs::ws2812::{PioWs2812, PioWs2812Program},
};
use embassy_time::Delay;
use embedded_hal_async::delay::DelayNs;
use rand_core::RngCore;
use smart_leds::{
    colors::{BLACK, BLUE_VIOLET, FIREBRICK, YELLOW},
    RGB8,
};
use tracker_mapper::{index_of, Coordinate};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

const NUM_LEDS: usize = 57;

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let mut rng = RoscRng;
    let mut config = config::Config::default();
    config.clocks = clocks::ClockConfig::rosc();

    let p = embassy_rp::init(config);

    let Pio {
        mut common, sm0, ..
    } = Pio::new(p.PIO0, Irqs);

    let ws2812_program = PioWs2812Program::new(&mut common);
    let mut led_strip = PioWs2812::new(&mut common, sm0, p.DMA_CH0, p.PIN_16, &ws2812_program);

    let mut leds = [RGB8::default(); NUM_LEDS];

    let nose = [Coordinate::new(0, 0), Coordinate::new(1, 8)];
    let mouth = [
        Coordinate::new(3, 7),
        Coordinate::new(3, 8),
        Coordinate::new(3, 9),
        Coordinate::new(3, 10),
        Coordinate::new(3, 11),
    ];
    let eyes = [
        // left.
        Coordinate::new(2, 15),
        Coordinate::new(2, 14),
        Coordinate::new(3, 14),
        // right.
        Coordinate::new(2, 1),
        Coordinate::new(2, 3),
        Coordinate::new(3, 2),
    ];

    paint(&mut leds, &nose, YELLOW);
    paint(&mut leds, &mouth, FIREBRICK);
    paint(&mut leds, &eyes, BLUE_VIOLET);

    led_strip.write(&leds).await;

    let mut delay = Delay;
    loop {
        let eyes_open = gen_range(&mut rng, 3000, 8000);
        delay.delay_ms(eyes_open).await;

        paint(&mut leds, &eyes, BLACK);

        let eyes_closed = gen_range(&mut rng, 200, 1200);
        delay.delay_ms(eyes_closed).await;
        paint(&mut leds, &eyes, BLUE_VIOLET);
    }
}

fn paint(leds: &mut [RGB8], coordinates: &[Coordinate], color: RGB8) {
    for coord in coordinates {
        let index = index_of(*coord) as usize;
        leds[index] = color;
    }
}

/// Random number within some range.
fn gen_range(rng: &mut RoscRng, min: u32, max: u32) -> u32 {
    let r = rng.next_u32();
    let range = max - min;
    let r = r % range;
    min + r
}
