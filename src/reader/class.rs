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

pub struct AttributeItem {
    pub name_index: u16,
    pub info: Vec<u8>
}

impl AttributeItem {
    pub fn new() -> AttributeItem {
        AttributeItem { name_index: 0, info: Vec::new() }
    }
}

pub struct FieldItem {
    pub access_flags: u16,
    pub name_index: u16,
    pub descriptor_index: u16,
    pub attributes: Vec<AttributeItem>
}

impl FieldItem {
    pub fn new() -> FieldItem {
        FieldItem { access_flags: 0, name_index: 0, descriptor_index: 0, attributes: Vec::new() }
    }
}

pub struct ClassResult {
    pub constant_pool: Vec<ConstantPoolItem>,
    pub access_flags: u16,
    pub this_class_index: u16,
    pub super_class_index: u16,
    pub interfaces: Vec<u16>,
    pub fields: Vec<FieldItem>,
    pub methods: Vec<FieldItem>,
    pub attributes: Vec<AttributeItem>
}

impl ClassResult {
    pub fn new() -> ClassResult {
        ClassResult {
            constant_pool: Vec::new(),
            access_flags: 0,
            this_class_index: 0,
            super_class_index: 0,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            attributes: Vec::new()
        }
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

fn read_attribute(reader: &mut Read) -> Result<AttributeItem, ClassReadError> {
    let mut attribute = AttributeItem::new();
    attribute.name_index = try!(reader.read_u16::<BigEndian>());
    let length = try!(reader.read_u32::<BigEndian>());
    try!(reader.take(length as u64).read_to_end(&mut attribute.info));
    println!("Attribute with name index {} length {:?}", attribute.name_index, attribute.info);
    return Ok(attribute);
}

fn read_field(reader: &mut Read) -> Result<FieldItem, ClassReadError> {
    let mut field = FieldItem::new();
    field.access_flags = try!(reader.read_u16::<BigEndian>());
    field.name_index = try!(reader.read_u16::<BigEndian>());
    field.descriptor_index = try!(reader.read_u16::<BigEndian>());

    println!("Field with name index {} descriptor index {}", field.name_index, field.descriptor_index);
    let attributes_count = try!(reader.read_u16::<BigEndian>());
    println!("Field has {} attributes", attributes_count);
    for i in 0..attributes_count {
        field.attributes.push(try!(read_attribute(reader)));
    }
    return Ok(field);
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
    let mut reader = BufReader::new(file);
    let magic = try!(reader.read_u32::<BigEndian>());
    let minor = try!(reader.read_u16::<BigEndian>());
    let major = try!(reader.read_u16::<BigEndian>());
    let version = (major as f32) + ((minor as f32) / (base(minor) as f32));

    if magic != 0xCAFEBABE {
        return Err(ClassReadError::Parse);
    }

    if major < 45 || major > 52 {
        return Err(ClassReadError::UnsupportedVersion(version));
    }

    let cp_count = try!(reader.read_u16::<BigEndian>());
    println!("cp: {}", cp_count);

    if cp_count == 0 {
        return Err(ClassReadError::Parse);
    }

    let mut ret = ClassResult::new();

    for i in 1..cp_count {
        ret.constant_pool.push(try!(read_constant_pool(&mut reader)));
    }

    ret.access_flags = try!(reader.read_u16::<BigEndian>());
    println!("access_flags: {}", ret.access_flags);
    ret.this_class_index = try!(reader.read_u16::<BigEndian>());
    ret.super_class_index = try!(reader.read_u16::<BigEndian>());
    println!("class_indexes: {} {}", ret.this_class_index, ret.super_class_index);

    let interfaces_count = try!(reader.read_u16::<BigEndian>());
    println!("Interface count: {}", interfaces_count);
    for i in 0..interfaces_count {
        ret.interfaces.push(try!(reader.read_u16::<BigEndian>()));
    }

    let fields_count = try!(reader.read_u16::<BigEndian>());
    println!("Fields count: {}", fields_count);
    for i in 0..fields_count {
        ret.fields.push(try!(read_field(&mut reader)));
    }

    let methods_count = try!(reader.read_u16::<BigEndian>());
    println!("Methods count: {}", methods_count);
    for i in 0..methods_count {
        ret.methods.push(try!(read_field(&mut reader)));
    }

    let attributes_count = try!(reader.read_u16::<BigEndian>());
    println!("Attributes count: {}", attributes_count);
    for i in 0..attributes_count {
        ret.attributes.push(try!(read_attribute(&mut reader)));
    }

    return Ok(ret);
}