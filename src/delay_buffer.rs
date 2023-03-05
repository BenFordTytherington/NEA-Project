pub struct DelayBuffer {
    buffer: Vec<f64>,
    index: usize,
}

impl DelayBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0.0; capacity],
            index: 0,
        }
    }

    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    pub fn write(&mut self, value: f64) {
        self.buffer[self.index] = value;

        // modulo used to wrap index to start of buffer once at the end
        self.index = (self.index + 1) % self.capacity();
    }

    pub fn read(&self, delay: usize) -> f64 {
        let offset = if delay < self.index {
            // -1 gives the last sample written to the buffer and then we get the sample from 'delay' number of samples previously
            self.index - 1 - delay
        } else {
            // if the delay is bigger than the current index, the previous operation overflows the start of the array.
            self.capacity() + self.index - 1 - delay
        };
        self.buffer[offset] // return the sample from the buffer at the offset.
    }
}

#[cfg(test)]
mod tests {
    use super::DelayBuffer;

    #[test]
    fn test_new() {
        let delay_buffer = DelayBuffer::new(3);
        assert_eq!(delay_buffer.buffer.capacity(), 3);
        assert_eq!(delay_buffer.buffer.len(), 3);
        assert_eq!(delay_buffer.index, 0);
    }

    #[test]
    fn test_write() {
        let mut delay_buffer = DelayBuffer::new(5);
        delay_buffer.write(1.0);
        delay_buffer.write(2.0);
        delay_buffer.write(3.0);
        delay_buffer.write(4.0);
        delay_buffer.write(5.0);
        assert_eq!(delay_buffer.buffer, vec![1.0, 2.0, 3.0, 4.0, 5.0])
    }

    #[test]
    fn test_write_wrap() {
        let mut delay_buffer = DelayBuffer::new(5);
        delay_buffer.write(1.0);
        delay_buffer.write(2.0);
        delay_buffer.write(3.0);
        delay_buffer.write(4.0);
        delay_buffer.write(5.0);
        delay_buffer.write(6.0);
        assert_eq!(delay_buffer.buffer, vec![6.0, 2.0, 3.0, 4.0, 5.0])
    }

    #[test]
    fn test_read() {
        let mut delay_buffer = DelayBuffer::new(5);
        delay_buffer.write(1.0);
        delay_buffer.write(2.0);
        delay_buffer.write(3.0);
        delay_buffer.write(4.0);
        delay_buffer.write(5.0);
        assert_eq!(delay_buffer.read(0), 5.0);
        assert_eq!(delay_buffer.read(1), 4.0);
        assert_eq!(delay_buffer.read(2), 3.0);
    }
}
