extern crate byteorder;

use std::path::Path;
use std::fs::File;
use std::io;
use std::io::Read;
use std::io::BufReader;
use self::byteorder::{BigEndian, ReadBytesExt};

#[derive(Debug)]
pub enum ClassReadError {
    Io(io::Error),
    Parse,
    UnsupportedVersion(f32)
}

impl From<io::Error> for ClassReadError {
    fn from(err: io::Error) -> ClassReadError {
        ClassReadError::Io(err)
    }
}

fn base(n: u16) -> u16 {
    let mut val = n;
    let mut out = 1;
    while val > 0 {
        val /= 10;
        out *= 10;
    }
    return out;
}

fn read_constant_pool(reader: &mut Read) -> Result<i32, ClassReadError> {
    let tag = try!(reader.read_u8());
    println!("tag: {}", tag);

    return Ok(0);
}

pub fn read(filename: &Path) -> Result<i32, ClassReadError> {
    let file = try!(File::open(filename));
    let mut buf_reader = BufReader::new(file);
    let magic = try!(buf_reader.read_u32::<BigEndian>());
    let minor = try!(buf_reader.read_u16::<BigEndian>());
    let major = try!(buf_reader.read_u16::<BigEndian>());
    let version = (major as f32) + ((minor as f32) / (base(minor) as f32));

    if magic != 0xCAFEBABE {
        return Err(ClassReadError::Parse);
    }

    if major < 45 || major > 52 {
        return Err(ClassReadError::UnsupportedVersion(version));
    }

    let cp_count = try!(buf_reader.read_u16::<BigEndian>());
    println!("cp: {}", cp_count);

    if cp_count == 0 {
        return Err(ClassReadError::Parse);
    }

    for i in 1..cp_count {
        try!(read_constant_pool(&mut buf_reader));
    }

    return Ok(0);
}