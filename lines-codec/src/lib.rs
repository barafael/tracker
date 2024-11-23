#![cfg_attr(not(test), no_std)]

mod read_line;
mod read_line_async;

pub struct ReadLine<R, const SIZE: usize> {
    source: R,
    buffer: rbf::RingBuffer<u8, SIZE>,
}

#[cfg(test)]
mod test {
    use super::*;

    use std::collections::VecDeque;

    use embedded_io::{ErrorType, Read};

    struct MockReader {
        data: VecDeque<Vec<u8>>,
        cursor: usize,
    }

    impl ErrorType for MockReader {
        type Error = rbf::Error;
    }

    impl Read for MockReader {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            let Some(data) = &self.data.pop_front() else {
                return Ok(0);
            };
            if self.cursor >= data.len() && self.data.is_empty() {
                return Ok(0);
            }
            let remaining = &data[self.cursor..];
            let to_read = buf.len().min(remaining.len());
            buf[..to_read].copy_from_slice(&remaining[..to_read]);
            self.cursor = 0;
            Ok(to_read)
        }
    }

    #[test]
    fn basic() {
        let data = vec![
            b"Hello\nWorld\nThis is a test\n".to_vec(),
            b"second write".to_vec(),
            b"second write".to_vec(),
            b"second write".to_vec(),
            b"ends here\n".to_vec(),
            b"\n".to_vec(),
        ]
        .into();
        let reader = MockReader { data, cursor: 0 };

        let mut line_reader = ReadLine::<_, 64>::new(reader);

        let mut line = [0u8; 1024];
        loop {
            match line_reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(n) => {
                    println!("Read line: {:?}", std::str::from_utf8(&line[..n]).unwrap());
                    assert!(&line[..n].ends_with(b"\n"));
                }
                Err(e) => eprintln!("error: {e:?}"),
            }
        }
    }
}
