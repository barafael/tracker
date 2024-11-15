#![no_std]
#![no_main]

use core::f32::consts::{PI, TAU};

use bno080::{interface::i2c::ALTERNATE_ADDRESS, wrapper::BNO080};
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts, i2c,
    peripherals::{I2C0, PIO0},
    pio::{InterruptHandler, Pio},
    pio_programs::ws2812::{PioWs2812, PioWs2812Program},
};
use embassy_time::{Delay, Duration, Ticker};
use num_quaternion::Q32;
use smart_leds::{
    colors::{self, BLACK},
    RGB8,
};
use tracker_mapper::{index_of, Coordinate};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
    I2C0_IRQ => i2c::InterruptHandler<I2C0>;
});

const NUM_LEDS: usize = 57;
const COLOR: RGB8 = colors::ORANGE_RED;
const LOOP_DURATION: Duration = Duration::from_millis(10);

#[inline(always)]
fn adjust_color_for_led_type(color: &mut RGB8) {
    #[cfg(feature = "sk6812")]
    core::mem::swap(&mut color.r, &mut color.g);
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let p = embassy_rp::init(Default::default());

    let sda = p.PIN_20;
    let scl = p.PIN_21;

    let i2c = i2c::I2c::new_async(p.I2C0, scl, sda, Irqs, i2c::Config::default());

    let interface = bno080::interface::I2cInterface::new(i2c, ALTERNATE_ADDRESS);

    let mut bno = BNO080::new_with_interface(interface);
    bno.init(&mut Delay).unwrap();

    bno.enable_rotation_vector(LOOP_DURATION.as_millis() as u16)
        .unwrap();

    let Pio {
        mut common, sm0, ..
    } = Pio::new(p.PIO0, Irqs);

    let ws2812_program = PioWs2812Program::new(&mut common);
    let mut led_strip = PioWs2812::new(&mut common, sm0, p.DMA_CH0, p.PIN_16, &ws2812_program);

    let mut leds = [RGB8::default(); NUM_LEDS];

    let mut ticker = Ticker::every(LOOP_DURATION);
    let mut color = COLOR;

    // make it nice orange color.
    color.g -= 40;

    adjust_color_for_led_type(&mut color);
    leds.iter_mut().for_each(|l| *l = BLACK);
    leds[56] = color;
    led_strip.write(&leds).await;
    loop {
        bno.handle_all_messages(&mut Delay, 1u8);
        let quat = bno.rotation_quaternion().unwrap();
        defmt::trace!("{:?}", quat);
        let quat = Q32::new(quat[0], quat[1], quat[2], quat[3]);
        let Some(quat) = quat.normalize() else {
            defmt::warn!("not normalizable");
            continue;
        };
        let euler = quat.to_euler_angles();
        // defmt::info!("{:?}", (euler.roll, euler.pitch, euler.yaw));

        let angle = euler.roll;
        let step = (angle + PI) / TAU;
        let step = step * 16.0;
        defmt::info!("step: {}", step);
        let step = step as u8;
        defmt::info!("step: {}", step);
        leds.iter_mut().for_each(|l| *l = BLACK);
        let coord = Coordinate::new(3, step);
        let index = index_of(coord);
        leds[index as usize] = color;
        led_strip.write(&leds).await;

        ticker.next().await;
    }
}
