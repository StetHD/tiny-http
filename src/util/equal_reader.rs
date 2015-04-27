use std::sync::mpsc::channel;
use std::io::Result as IoResult;
use std::sync::mpsc::{Sender, Receiver};
use std::io::Read;
use std::mem;

/// A `Reader` that reads exactly the number of bytes from a sub-reader.
/// 
/// If the limit is reached, it returns EOF. If the limit is not reached
/// when the destructor is called, the remaining bytes will be read and
/// thrown away.
pub struct EqualReader<R> where R: Read {
    reader: R,
    size: usize,
    last_read_signal: Sender<IoResult<()>>,
}

impl<R> EqualReader<R> where R: Read {
    pub fn new(reader: R, size: usize) -> (EqualReader<R>, Receiver<IoResult<()>>) {
        let (tx, rx) = channel();

        let r = EqualReader {
            reader: reader,
            size: size,
            last_read_signal: tx,
        };

        (r, rx)
    }
}

impl<R> Read for EqualReader<R> where R: Read {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        if self.size == 0 {
            return Ok(0);
        }

        let buf = if buf.len() < self.size {
            buf
        } else {
            &mut buf[.. self.size]
        };

        self.reader.read(buf)
    }
}

impl<R> Drop for EqualReader<R> where R: Read {
    fn drop(&mut self) {
        let mut remaining_to_read = self.size;

        while remaining_to_read > 0 {
            let mut buf = vec![0 ; remaining_to_read];

            match self.reader.read(&mut buf) {
                Err(e) => { self.last_read_signal.send(Err(e)).ok(); break; }
                Ok(0) => { self.last_read_signal.send(Ok(())).ok(); break; },
                Ok(other) => { remaining_to_read -= other; }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EqualReader;

    #[test]
    fn test_limit() {
        use std::io::Cursor;

        let mut org_reader = Cursor::new("hello world".to_string().into_bytes());

        {
            let (mut equal_reader, _) = EqualReader::new(org_reader.by_ref(), 5);

            assert_eq!(equal_reader.read_to_string().unwrap(), "hello");
        }

        assert_eq!(org_reader.read_to_string().unwrap(), " world");
    }

    #[test]
    fn test_not_enough() {
        use std::io::Cursor;

        let mut org_reader = Cursor::new("hello world".to_string().into_bytes());

        {
            let (mut equal_reader, _) = EqualReader::new(org_reader.by_ref(), 5);

            assert_eq!(equal_reader.read_u8().unwrap(), b'h');
        }

        assert_eq!(org_reader.read_to_string().unwrap(), " world");
    }
}
