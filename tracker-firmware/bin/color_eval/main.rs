#![no_std]
#![no_main]

mod color_wheel;

use bno080::interface::i2c::DEFAULT_ADDRESS;
use bno080::wrapper::BNO080;
use color_wheel::{COLORS, COLOR_NAMES};
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::i2c::InterruptHandler as I2cInterruptHandler;
use embassy_rp::peripherals::{I2C1, PIO0};
use embassy_rp::pio::{InterruptHandler as PioInterruptHandler, Pio};
use embassy_rp::pio_programs::ws2812::{PioWs2812, PioWs2812Program};
use embassy_rp::{bind_interrupts, i2c};
use embassy_time::{Delay, Duration, Ticker};
use smart_leds::RGB8;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    I2C1_IRQ => I2cInterruptHandler<I2C1>;
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
});

const NUM_LEDS: usize = 10;
const LOOP_DURATION: Duration = Duration::from_millis(1000);

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

    let sda = p.PIN_14;
    let scl = p.PIN_15;

    let i2c = i2c::I2c::new_async(p.I2C1, scl, sda, Irqs, i2c::Config::default());

    let interface = bno080::interface::I2cInterface::new(i2c, DEFAULT_ADDRESS);

    let mut bno = BNO080::new_with_interface(interface);
    bno.init(&mut Delay).unwrap();

    bno.enable_rotation_vector(LOOP_DURATION.as_millis() as u16)
        .unwrap();

    loop {
        println!("loop");
        for i in 0..COLORS.len() {
            bno.handle_all_messages(&mut Delay, 1u8);
            let quat = bno.rotation_quaternion().unwrap();
            defmt::println!("{}", quat);

            let color = COLORS[i];
            let name = COLOR_NAMES[i];
            println!("{}", name);
            leds.iter_mut().for_each(|led| *led = color);

            led_strip.write(&leds).await;

            ticker.next().await;
        }
    }
}
