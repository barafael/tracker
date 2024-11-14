#![cfg_attr(not(test), no_std)]

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Coordinate {
    ring: u8,
    step: u8,
}

pub const NUM_LEDS: usize = 57;

pub static ARRAY: [[u8; STEP_COUNT]; RING_COUNT] = make_table();

impl Coordinate {
    pub const fn new(ring: u8, step: u8) -> Self {
        Self { ring, step }
    }

    pub fn from_world_coordinates(distance: u8, angle: u16) -> Self {
        let ring = distance.min(4);
        let angle = angle % 360;
        // let step = ((angle as f32 / 360.0) * 16.0).round() as u16;
        let step = ((angle * 16) + 180) / 360;
        let step = (step % 16) as u8;
        Self { ring, step }
    }
}

pub const RING_COUNT: usize = 5;
pub const STEP_COUNT: usize = 16;

const fn make_table() -> [[u8; STEP_COUNT]; RING_COUNT] {
    let mut array: [[u8; STEP_COUNT]; RING_COUNT] = [[0u8; STEP_COUNT]; RING_COUNT];
    let mut ring = 0;
    let mut step = 0;
    loop {
        loop {
            array[ring as usize][step as usize] = index_of(Coordinate::new(ring, step));
            step += 1;
            if step == STEP_COUNT as u8 {
                break;
            }
        }
        ring += 1;
        step = 0;
        if ring == RING_COUNT as u8 {
            break;
        }
    }
    array
}

pub const fn index_of(coordinate: Coordinate) -> u8 {
    let virtual_index = virtual_index_of(coordinate);
    devirtualize_led_index(virtual_index)
}

const fn virtual_index_of(Coordinate { ring, step }: Coordinate) -> u8 {
    let result = ring * 16 + step;
    79 - result
}

const fn devirtualize_led_index(virtual_index: u8) -> u8 {
    let virtual_index = virtual_index % 80;
    match virtual_index {
        0..48 => virtual_index,
        48..64 => {
            let index = virtual_index - 48;
            let index = index / 2;
            index + 48
        }
        64..80 => 56,
        _ => panic!("a"),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use test_case::test_case;

    #[test_case(0, 0 => Coordinate::new(0, 0))]
    #[test_case(1, 45 => Coordinate::new(1, 2))]
    #[test_case(4, 180 => Coordinate::new(4, 8))]
    #[test_case(4, 179 => Coordinate::new(4, 8))]
    #[test_case(4, 181 => Coordinate::new(4, 8))]
    #[test_case(4, 359 => Coordinate::new(4, 0))]
    fn makes_coordinate(distance: u8, angle: u16) -> Coordinate {
        Coordinate::from_world_coordinates(distance, angle)
    }

    #[test_case(Coordinate::new(0, 0) => 79)]
    #[test_case(Coordinate::new(4, 15) => 0)]
    #[test_case(Coordinate::new(4, 12) => 3)]
    #[test_case(Coordinate::new(4, 0) => 15)]
    #[test_case(Coordinate::new(4, 4) => 11)]
    #[test_case(Coordinate::new(1, 7) => 56)]
    #[test_case(Coordinate::new(1, 8) => 55)]
    #[test_case(Coordinate::new(1, 9) => 54)]
    #[test_case(Coordinate::new(1, 0) => 63)]
    #[test_case(Coordinate::new(1, 1) => 62)]
    fn calculates_virtual_index_of_led(coordinate: Coordinate) -> u8 {
        virtual_index_of(coordinate)
    }

    #[test_case(0 => 0)]
    #[test_case(4 => 4)]
    #[test_case(47 => 47)]
    #[test_case(48 => 48)]
    #[test_case(49 => 48)]
    #[test_case(63 => 55)]
    #[test_case(64 => 56)]
    #[test_case(72 => 56)]
    #[test_case(79 => 56)]
    #[test_case(80 => 0)]
    #[test_case(160 => 0)]
    fn devirtualizes_led_index(virtual_index: u8) -> u8 {
        devirtualize_led_index(virtual_index)
    }

    #[test]
    fn index_of_and_table_are_equivalent() {
        let table = make_table();

        dbg!(std::mem::size_of_val(&table));

        for ring in 0..RING_COUNT {
            for step in 0..STEP_COUNT {
                assert_eq!(
                    table[ring][step],
                    index_of(Coordinate::new(ring as u8, step as u8))
                );
            }
        }
    }
}
