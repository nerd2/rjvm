extern crate byteorder;

use std::path::Path;
use std::fs::File;
use std::str;
use std::io;
use std::io::Read;
use std::io::BufReader;
use std::string::FromUtf8Error;
use std::string::String;
use self::byteorder::{BigEndian, ReadBytesExt};

#[derive(Debug)]
pub enum ClassReadError {
    Io(io::Error),
    Parse,
    UnsupportedVersion(f32)
}

pub enum ConstantPoolItem {
    CONSTANT_Utf8(String),
    CONSTANT_Class{index: u16},
    CONSTANT_String{index: u16},
    CONSTANT_Fieldref{class_index: u16, name_and_type_index: u16},
    CONSTANT_Methodref{class_index: u16, name_and_type_index: u16},
    CONSTANT_NameAndType{name_index: u16, descriptor_index: u16},
}

pub struct ClassResult {
    pub constant_pool: Vec<ConstantPoolItem>
}

impl ClassResult {
    pub fn new() -> ClassResult {
        ClassResult { constant_pool: Vec::new() }
    }
}

impl From<io::Error> for ClassReadError {
    fn from(err: io::Error) -> ClassReadError {
        ClassReadError::Io(err)
    }
}

impl From<FromUtf8Error> for ClassReadError {
    fn from(err: FromUtf8Error) -> ClassReadError {
        ClassReadError::Parse
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

fn read_constant_pool(reader: &mut Read) -> Result<ConstantPoolItem, ClassReadError> {
    let tag = try!(reader.read_u8());
    match tag {
        1 => {
            // CONSTANT_Utf8
            let length = try!(reader.read_u16::<BigEndian>());
            let mut buf: Vec<u8> = Vec::new();
            try!(reader.take(length as u64).read_to_end(&mut buf));
            let string = try!(String::from_utf8(buf));
            println!("UTF8 {} '{}'", length, string);
            return Ok(ConstantPoolItem::CONSTANT_Utf8(string));
        }
        7 => {
            // CONSTANT_Class
            let class_index = try!(reader.read_u16::<BigEndian>());
            println!("Class ref {}", class_index);
            return Ok(ConstantPoolItem::CONSTANT_Class{index: class_index});
        },
        8 => {
            // CONSTANT_String
            let string_index = try!(reader.read_u16::<BigEndian>());
            println!("String ref {}", string_index);
            return Ok(ConstantPoolItem::CONSTANT_String{index:string_index});
        },
        9 => {
            // CONSTANT_Fieldref
            let class_index = try!(reader.read_u16::<BigEndian>());
            let name_and_type_index = try!(reader.read_u16::<BigEndian>());
            println!("Field ref {} {}", class_index, name_and_type_index);
            return Ok(ConstantPoolItem::CONSTANT_Fieldref{class_index: class_index, name_and_type_index: name_and_type_index});
        },
        10 => {
            // CONSTANT_Methodref
            let class_index = try!(reader.read_u16::<BigEndian>());
            let name_and_type_index = try!(reader.read_u16::<BigEndian>());
            println!("Method ref {} {}", class_index, name_and_type_index);
            return Ok(ConstantPoolItem::CONSTANT_Methodref{class_index: class_index, name_and_type_index: name_and_type_index});
        },
        12 => {
            // CONSTANT_NameAndType
            let name_index = try!(reader.read_u16::<BigEndian>());
            let descriptor_index = try!(reader.read_u16::<BigEndian>());
            println!("NameAndType {} {}", name_index, descriptor_index);
            return Ok(ConstantPoolItem::CONSTANT_NameAndType{name_index: name_index, descriptor_index: descriptor_index});
        }
        _ => {
            println!("unknown tag: {}", tag);
            return Err(ClassReadError::Parse);
        }
    }
}

pub fn read(filename: &Path) -> Result<ClassResult, ClassReadError> {
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

    let mut ret = ClassResult::new();

    for i in 1..cp_count {
        ret.constant_pool.push(try!(read_constant_pool(&mut buf_reader)));
    }

    return Ok(ret);
}