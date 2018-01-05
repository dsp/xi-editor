#[macro_use]
extern crate lazy_static;
extern crate serde;
extern crate serde_json;
use std::sync::{Mutex};
use serde::ser::{Serialize, Serializer};
//use serde::de::{Deserialize};
use std::iter::Extend;

struct Replay {
    enabled: bool,
    bufsize: usize,
    buffer: Mutex<Vec<u8>>,
}

impl Replay {
    pub fn disabled(bufsize: usize) -> Replay
    {
        Replay {
            enabled: false,
            bufsize: bufsize,
            buffer: Mutex::new(Vec::with_capacity(bufsize)),
        }
    }

    pub fn record<T>(&self, frame: &T)
    where T: Serialize
    {
        if self.enabled { 
            let mut buf = self.buffer.lock().unwrap();
            match serde_json::to_vec(&frame) {
                Ok(serialized) => {
                    buf.extend(serialized);
                },
                Err(_) => (),
            }
        }
    }
}

lazy_static! {
    static ref REPLAY : Replay = Replay::disabled(4096);
}

pub fn record<T: Serialize>(frame: &T) {
    REPLAY.record(frame);
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
