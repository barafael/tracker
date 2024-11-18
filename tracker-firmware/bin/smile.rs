#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    clocks::RoscRng,
    config,
    peripherals::PIO0,
    pio::{InterruptHandler, Pio},
    pio_programs::ws2812::{PioWs2812, PioWs2812Program},
};
use embassy_time::Delay;
use embedded_hal_async::delay::DelayNs;
use rand_core::RngCore;
use smart_leds::{
    colors::{BLUE_VIOLET, DARK_SLATE_GRAY, FIREBRICK, GAINSBORO, YELLOW},
    RGB8,
};
use tracker_firmware::adjust_color_for_led_type;
use tracker_mapper::{index_of, Coordinate};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

const NUM_LEDS: usize = 57;

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let config = config::Config::default();

    let p = embassy_rp::init(config);

    let Pio {
        mut common, sm0, ..
    } = Pio::new(p.PIO0, Irqs);

    let ws2812_program = PioWs2812Program::new(&mut common);
    let mut led_strip = PioWs2812::new(&mut common, sm0, p.DMA_CH0, p.PIN_16, &ws2812_program);

    let mut leds = [RGB8::default(); NUM_LEDS];

    let mut rng = RoscRng;

    let nose = [
        Coordinate::new(0, 0),
        Coordinate::new(1, 8),
        Coordinate::new(1, 6),
        Coordinate::new(1, 10),
    ];
    let mouth = [
        Coordinate::new(3, 5),
        Coordinate::new(3, 6),
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
        Coordinate::new(2, 2),
        Coordinate::new(2, 3),
        Coordinate::new(3, 3),
    ];

    let nose_color = adjust_color_for_led_type(YELLOW);
    let mouth_color = adjust_color_for_led_type(FIREBRICK);
    let eye_color = adjust_color_for_led_type(GAINSBORO);

    paint(&mut leds, &nose, nose_color);
    paint(&mut leds, &mouth, mouth_color);
    paint(&mut leds, &eyes, eye_color);

    let mut delay = Delay;
    loop {
        led_strip.write(&leds).await;
        let eyes_open = gen_range(&mut rng, 5000, 8000);
        defmt::trace!("eyes open for {}ms", eyes_open);
        delay.delay_ms(eyes_open).await;

        paint(&mut leds, &eyes, DARK_SLATE_GRAY / 2);
        led_strip.write(&leds).await;

        let eyes_closed = gen_range(&mut rng, 100, 800);
        defmt::trace!("eyes closing for {}ms", eyes_closed);
        delay.delay_ms(eyes_closed).await;
        paint(&mut leds, &eyes, BLUE_VIOLET);
    }
}

fn paint(leds: &mut [RGB8], coordinates: &[Coordinate], color: RGB8) {
    for coord in coordinates {
        let index = index_of(*coord) as usize;
        let mut color = color;
        color /= 9;
        leds[index] = color;
    }
}

/// Random number within some range.
fn gen_range(rng: &mut RoscRng, min: u32, max: u32) -> u32 {
    let r = rng.next_u32();
    defmt::trace!("{}", r);
    let range = max - min;
    let r = r % range;
    min + r
}
