use embedded_io::Read;

use crate::ReadLine;

impl<R: embedded_io_async::Read, const SIZE: usize> ReadLine<R, SIZE> {
    pub fn new_async(source: R) -> Self {
        Self {
            source,
            buffer: rbf::RingBuffer::<u8, SIZE>::new(),
        }
    }

    pub async fn read_line_async(&mut self, buf: &mut [u8]) -> Result<usize, rbf::Error> {
        loop {
            // Check if the buffer contains a newline
            if let Some(pos) = self.buffer.iter().position(|&b| b == b'\n') {
                let len = self.buffer.read(&mut buf[..=pos])?;
                return Ok(len);
            }

            // Otherwise, read more data from the source
            // let bytes_read = self.source.read(buf).map_err(|e| rbf::Error::BufferFull)?;
            let Ok(bytes_read) = self.source.read(buf).await else {
                continue;
            };
            if bytes_read == 0 {
                // EOF reached
                if self.buffer.is_empty() {
                    return Ok(0); // No more data to read
                }
                // Return the remaining data as the last line
                let count = self.buffer.read(buf)?;
                return Ok(count);
            }

            // Write data into the ring buffer
            for byte in &buf[..bytes_read] {
                self.buffer.push_unless_full(*byte)?;
            }
        }
    }
}
