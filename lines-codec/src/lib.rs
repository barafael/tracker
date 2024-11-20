#![cfg_attr(not(test), no_std)]

use embedded_io::Read;

pub struct ReadLine<R, const SIZE: usize> {
    source: R,
    buffer: rbf::RingBuffer<u8, SIZE>,
}

impl<R: Read, const SIZE: usize> ReadLine<R, SIZE> {
    pub fn new(source: R) -> Self {
        Self {
            source,
            buffer: rbf::RingBuffer::<u8, SIZE>::new(),
        }
    }

    pub fn read_line(&mut self, buf: &mut [u8]) -> Result<usize, rbf::Error> {
        loop {
            // Check if the buffer contains a newline
            if let Some(pos) = self.buffer.iter().position(|&b| b == b'\n') {
                let len = self.buffer.read(&mut buf[..=pos])?;
                return Ok(len);
            }

            // Otherwise, read more data from the source
            let bytes_read = self.source.read(buf).unwrap();
            if bytes_read == 0 {
                // EOF reached
                if self.buffer.is_empty() {
                    return Ok(0); // No more data to read
                } else {
                    // Return the remaining data as the last line
                    let count = self.buffer.read(buf)?;
                    return Ok(count);
                }
            }

            // Write data into the ring buffer
            for byte in &buf[..bytes_read] {
                self.buffer.push_unless_full(*byte)?;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use core::str;
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
                    println!("Read line: {:?}", str::from_utf8(&line[..n]).unwrap());
                    assert!(&line[..n].ends_with(b"\n"));
                }
                Err(e) => eprintln!("error: {e:?}"),
            }
        }
    }
}
