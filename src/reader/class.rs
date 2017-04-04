extern crate byteorder;

use std::path::Path;
use std::fs::File;
use std::io::BufReader;
use self::byteorder::{BigEndian, ReadBytesExt};

pub fn read(filename: &Path) {
    let mut file = File::open(filename).unwrap();
    let mut buf_reader = BufReader::new(file);
    println!("Hello, world {}", buf_reader.read_u32::<BigEndian>().unwrap());
}