#![cfg_attr(not(test), no_std)]
#![no_main]

use core::f32::consts::{PI, TAU};

use core::{
    clone::Clone,
    default::Default,
    iter::{FromIterator, Iterator},
    marker::{Copy, Sized},
    module_path,
    option::Option::Some,
    prelude::rust_2021::derive,
    result::Result::Ok,
};

use bno080::{
    interface::{i2c::ALTERNATE_ADDRESS, I2cInterface},
    wrapper::BNO080,
};
use lines_codec::ReadLine;
use num_quaternion::Q32;

use defmt::unwrap;
use {defmt_rtt as _, panic_probe as _};

use embassy_executor::Executor;
use embassy_futures::select::{self, Either};
use embassy_rp::peripherals::UART0;
use embassy_rp::{
    bind_interrupts, config,
    i2c::{self, I2c},
    multicore::{spawn_core1, Stack},
    peripherals::{I2C0, PIO0},
    pio::{InterruptHandler, Pio},
    pio_programs::ws2812::{PioWs2812, PioWs2812Program},
    uart::{self, BufferedInterruptHandler, BufferedUart},
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Delay, Duration, Ticker};

use smart_leds::{
    colors::{self, BLACK},
    RGB8,
};

use static_cell::StaticCell;

use tiny_nmea::NMEA;

use tracker_firmware::adjust_color_for_led_type;
use tracker_mapper::{index_of, Coordinate};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
    I2C0_IRQ => i2c::InterruptHandler<I2C0>;
    UART0_IRQ => BufferedInterruptHandler<UART0>;
});

const NUM_LEDS: usize = 57;
const COLOR: RGB8 = colors::ORANGE_RED;
const BNO_UPDATE_PERIOD: Duration = Duration::from_millis(10);

const UART_BUFFER_SIZE: usize = 256;

static STEP: Signal<CriticalSectionRawMutex, u8> = Signal::new();
static NMEA: Signal<CriticalSectionRawMutex, NMEA> = Signal::new();

static mut CORE1_STACK: Stack<{ 4096 * 8 }> = Stack::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::println!("Hi! I'm your tracker.");
    let p = embassy_rp::init(config::Config::default());

    // IMU driver.
    let sda = p.PIN_20;
    let scl = p.PIN_21;
    let i2c = i2c::I2c::new_async(p.I2C0, scl, sda, Irqs, i2c::Config::default());
    let interface = bno080::interface::I2cInterface::new(i2c, ALTERNATE_ADDRESS);
    let mut bno = BNO080::new_with_interface(interface);
    bno.init(&mut Delay).unwrap();
    bno.enable_rotation_vector(BNO_UPDATE_PERIOD.as_millis() as u16)
        .unwrap();

    // Core 1 runs IMU only.
    spawn_core1(
        p.CORE1,
        unsafe { &mut *core::ptr::addr_of_mut!(CORE1_STACK) },
        move || {
            let executor1 = EXECUTOR1.init(Executor::new());
            executor1.run(|spawner| unwrap!(spawner.spawn(monitor_bno(bno))));
        },
    );

    // Led strip driver.
    let Pio {
        mut common, sm0, ..
    } = Pio::new(p.PIO0, Irqs);
    let ws2812_program: PioWs2812Program<'static, PIO0> = PioWs2812Program::new(&mut common);
    let led_strip = PioWs2812::new(&mut common, sm0, p.DMA_CH0, p.PIN_16, &ws2812_program);
    let leds = [RGB8::default(); NUM_LEDS];

    // NMEA over UART message reader.
    static TX_BUF: StaticCell<[u8; UART_BUFFER_SIZE]> = StaticCell::new();
    let tx_buf = &mut TX_BUF.init([0; UART_BUFFER_SIZE])[..];
    static RX_BUF: StaticCell<[u8; UART_BUFFER_SIZE]> = StaticCell::new();
    let rx_buf = &mut RX_BUF.init([0; UART_BUFFER_SIZE])[..];

    let mut config = uart::Config::default();
    config.baudrate = 9600;
    let uart = BufferedUart::new(p.UART0, Irqs, p.PIN_0, p.PIN_1, tx_buf, rx_buf, config);

    let reader = lines_codec::ReadLine::<_, UART_BUFFER_SIZE>::new(uart);

    // Core 0 runs GPS and main loop with LED update logic.
    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| {
        unwrap!(spawner.spawn(main_task(led_strip, leds)));
        unwrap!(spawner.spawn(monitor_gps(reader)));
    });
}

#[embassy_executor::task]
async fn main_task(
    mut led_strip: PioWs2812<'static, PIO0, 0, NUM_LEDS>,
    mut leds: [RGB8; NUM_LEDS],
) {
    let mut color = adjust_color_for_led_type(COLOR);
    // make it nice orange color.
    color.g -= 40;

    loop {
        let next = select::select(STEP.wait(), NMEA.wait()).await;
        match next {
            Either::First(step) => {
                let coord = Coordinate::new(3, step);
                // calculate led strip index
                let index = index_of(coord);
                // clear
                leds.iter_mut().for_each(|l| *l = BLACK);
                leds[index as usize] = color;

                // update LEDs
                led_strip.write(&leds).await;
            }
            Either::Second(nmea) => {
                defmt::println!("{:?}", nmea);
            }
        }
    }
}

#[embassy_executor::task]
async fn monitor_gps(mut reader: ReadLine<BufferedUart<'static, UART0>, UART_BUFFER_SIZE>) {
    let mut nmea = tiny_nmea::NMEA::new();
    let mut line = [0u8; UART_BUFFER_SIZE];

    loop {
        let Ok(len) = reader
            .read_line_async(&mut line)
            .await
            .inspect_err(|e| defmt::warn!("{}", e))
        else {
            continue;
        };
        defmt::trace!("{}", core::str::from_utf8(&line[..len]).ok());

        let s = heapless::String::from_iter(line[..len].iter().map(|c| *c as char));
        let _ = nmea.update(&s).map_err(|()| defmt::warn!("parser error"));

        defmt::println!("{:?}", nmea);
    }
}

#[embassy_executor::task]
async fn monitor_bno(mut bno: BNO080<I2cInterface<I2c<'static, I2C0, i2c::Async>>>) {
    defmt::println!("monitoring bno080");
    let mut last_step = 255;
    let mut ticker = Ticker::every(BNO_UPDATE_PERIOD);
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
        let step = step as u8;
        defmt::info!("step: {}", step);

        if step != last_step {
            STEP.signal(step);
            last_step = step;
        }

        ticker.next().await;
    }
}
