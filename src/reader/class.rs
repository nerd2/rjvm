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

pub struct ExceptionItem {
    start_pc: u16,
    end_pc: u16,
    handler_pc: u16,
    catch_type: u16
}

pub enum AttributeItem {
    ConstantValue{index: u16},
    Code{max_stack: u16, max_locals: u16, code: Vec<u8>, exceptions: Vec<ExceptionItem>, attributes: Vec<AttributeItem>},
    Exceptions{indicies: Vec<u16>},
    Unknown{name_index: u16, info: Vec<u8>}
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

fn read_exception(reader: &mut Read) -> Result<ExceptionItem, ClassReadError> {
    let start_pc = try!(reader.read_u16::<BigEndian>());
    let end_pc = try!(reader.read_u16::<BigEndian>());
    let handler_pc = try!(reader.read_u16::<BigEndian>());
    let catch_type = try!(reader.read_u16::<BigEndian>());
    return Ok(ExceptionItem {start_pc: start_pc, end_pc: end_pc, handler_pc: handler_pc, catch_type: catch_type});
}

fn get_cp_str(cp: &Vec<ConstantPoolItem>, index:u16) -> Result<&str, ClassReadError> {
    if index == 0 || index as usize > cp.len() {
        return Err(ClassReadError::Parse);
    } else {
        match cp[(index - 1) as usize] {
            ConstantPoolItem::CONSTANT_Utf8(ref s) => {
                return Ok(&s);
            }
            _ => {
                return Err(ClassReadError::Parse);
            }
        }
    }
}

fn read_attribute(cp: &Vec<ConstantPoolItem>, reader: &mut Read) -> Result<AttributeItem, ClassReadError> {
    let name_index = try!(reader.read_u16::<BigEndian>());
    let length = try!(reader.read_u32::<BigEndian>());
    match try!(get_cp_str(cp, name_index)) {
        "ConstantValue" => {
            if length != 2 {
                return Err(ClassReadError::Parse);
            } else {
                let index = try!(reader.read_u16::<BigEndian>());
                println!("ConstantValue attribute with index of {}", index);
                return Ok(AttributeItem::ConstantValue {index: index});
            }
        }
        "Code" => {
            let max_stack = try!(reader.read_u16::<BigEndian>());
            let max_locals = try!(reader.read_u16::<BigEndian>());
            let code_length = try!(reader.read_u32::<BigEndian>());
            let mut code = Vec::new();
            try!(reader.take(code_length as u64).read_to_end(&mut code));
            let exception_table_length =  try!(reader.read_u16::<BigEndian>());
            let mut exceptions = Vec::new();
            for _ in 0..exception_table_length {
                exceptions.push(try!(read_exception(reader)));
            }
            let attributes_count = try!(reader.read_u16::<BigEndian>());
            let mut attributes = Vec::new();
            for _ in 0..attributes_count {
                attributes.push(try!(read_attribute(cp, reader)));
            }
            println!("Code attribute with {}B of code, {} exceptions and {} attributes", code_length, exception_table_length, attributes_count);

            return Ok(AttributeItem::Code {
                max_stack: max_stack, max_locals: max_locals, code: code, exceptions: exceptions, attributes: attributes
            })
        }
        "Exceptions" => {
            let num_exceptions = try!(reader.read_u16::<BigEndian>());
            let mut indicies = Vec::new();
            for _ in 0..num_exceptions {
                indicies.push(try!(reader.read_u16::<BigEndian>()));
            }
            println!("Exceptions attribute with {} indicies", num_exceptions);

            return Ok(AttributeItem::Exceptions {indicies: indicies})
        }
        _ => {
            let mut info = Vec::new();
            try!(reader.take(length as u64).read_to_end(&mut info));
            println!("Unknown attribute with name index {} data {:?}", name_index, info);
            return Ok(AttributeItem::Unknown {name_index: name_index, info: info});
        }
    }
}

fn read_field(cp: &Vec<ConstantPoolItem>, reader: &mut Read) -> Result<FieldItem, ClassReadError> {
    let mut field = FieldItem::new();
    field.access_flags = try!(reader.read_u16::<BigEndian>());
    field.name_index = try!(reader.read_u16::<BigEndian>());
    field.descriptor_index = try!(reader.read_u16::<BigEndian>());

    println!("Field with name index {} descriptor index {}", field.name_index, field.descriptor_index);
    let attributes_count = try!(reader.read_u16::<BigEndian>());
    println!("Field has {} attributes", attributes_count);
    for _ in 0..attributes_count {
        field.attributes.push(try!(read_attribute(cp, reader)));
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

    for _ in 1..cp_count {
        ret.constant_pool.push(try!(read_constant_pool(&mut reader)));
    }

    ret.access_flags = try!(reader.read_u16::<BigEndian>());
    println!("access_flags: {}", ret.access_flags);
    ret.this_class_index = try!(reader.read_u16::<BigEndian>());
    ret.super_class_index = try!(reader.read_u16::<BigEndian>());
    println!("class_indexes: {} {}", ret.this_class_index, ret.super_class_index);

    let interfaces_count = try!(reader.read_u16::<BigEndian>());
    println!("Interface count: {}", interfaces_count);
    for _ in 0..interfaces_count {
        ret.interfaces.push(try!(reader.read_u16::<BigEndian>()));
    }

    let fields_count = try!(reader.read_u16::<BigEndian>());
    println!("Fields count: {}", fields_count);
    for _ in 0..fields_count {
        ret.fields.push(try!(read_field(&ret.constant_pool, &mut reader)));
    }

    let methods_count = try!(reader.read_u16::<BigEndian>());
    println!("Methods count: {}", methods_count);
    for _ in 0..methods_count {
        ret.methods.push(try!(read_field(&ret.constant_pool, &mut reader)));
    }

    let attributes_count = try!(reader.read_u16::<BigEndian>());
    println!("Attributes count: {}", attributes_count);
    for _ in 0..attributes_count {
        ret.attributes.push(try!(read_attribute(&ret.constant_pool, &mut reader)));
    }

    return Ok(ret);
}