#![cfg_attr(not(test), no_std)]

use smart_leds::RGB8;

#[inline]
pub fn adjust_color_for_led_type(color: &mut RGB8) {
    #[cfg(feature = "sk6812")]
    core::mem::swap(&mut color.r, &mut color.g);
}
