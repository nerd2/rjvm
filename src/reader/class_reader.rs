#![deny(
unreachable_code,
unused_assignments,
unused_imports,
unused_variables,
)]

extern crate byteorder;

pub use reader::types::constant_pool::*;
use std::char;
use std::path::Path;
use std::fs::File;
use std::str;
use std::mem::transmute;
use std::io;
use std::io::Read;
use std::io::BufReader;
use std::string::String;
use std::rc::Rc;

use self::byteorder::{BigEndian, ReadBytesExt};

macro_rules! PRINT_LEVEL { () => {0} }

macro_rules! debugPrint {
    ($enabled:expr, $level:expr, $fmt:expr) => {{if $enabled && $level <= PRINT_LEVEL!() { println!($fmt); } }};
    ($enabled:expr, $level:expr, $fmt:expr, $($arg:tt)*) => {{if $enabled && $level <= PRINT_LEVEL!() { println!($fmt, $($arg)*); } }};
}

#[derive(Debug)]
pub enum ClassReadError {
    Io(io::Error),
    Parse,
    Parse2(String),
    NativeMethod(String),
    UTF8Error(String),
    UnsupportedVersion(f32)
}

#[derive(Clone, Debug, PartialEq)]
#[allow(non_camel_case_types)]
pub enum ConstantPoolItem {
    CONSTANT_Utf8(Rc<String>),
    CONSTANT_Class{index: u16},
    CONSTANT_Integer{value: u32},
    CONSTANT_Long{value: u64},
    CONSTANT_Float{value: f32},
    CONSTANT_Double{value: f64},
    CONSTANT_String{index: u16},
    CONSTANT_Fieldref{class_index: u16, name_and_type_index: u16},
    CONSTANT_Methodref{class_index: u16, name_and_type_index: u16},
    CONSTANT_NameAndType{name_index: u16, descriptor_index: u16},
    CONSTANT_InterfaceMethodref{class_index: u16, name_and_type_index: u16},
    CONSTANT_MethodHandle{reference_kind: u8, reference_index: u16},
    CONSTANT_MethodType{descriptor_index: u16},
    CONSTANT_InvokeDynamic{bootstrap_method_attr_index: u16, name_and_type_index: u16},
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExceptionItem {
    pub start_pc: u16,
    pub end_pc: u16,
    pub handler_pc: u16,
    pub catch_type: u16
}

#[derive(Clone, Debug, PartialEq)]
pub struct Code {
    pub max_stack: u16,
    pub max_locals: u16,
    pub code: Vec<u8>,
    pub exceptions: Vec<ExceptionItem>,
    pub attributes: Vec<AttributeItem>
}

impl Code {
    pub fn new() -> Code {
        Code { max_locals: 0, max_stack: 0, code: Vec::new(), exceptions: Vec::new(), attributes: Vec::new() }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AttributeItem {
    ConstantValue{index: u16},
    Code(Code),
    Signature{index: u16},
    Exceptions{indicies: Vec<u16>},
    Unknown{name_index: u16, info: Vec<u8>}
}

pub const ACC_PUBLIC: u16 = 0x0001;
pub const ACC_PRIVATE: u16 = 0x0002;
pub const ACC_PROTECTED: u16 = 0x0004;
pub const ACC_STATIC: u16 = 0x0008;
pub const ACC_FINAL: u16 = 0x0010;
pub const ACC_VOLATILE: u16 = 0x0040;
pub const ACC_TRANSIENT: u16 = 0x0080;
pub const ACC_NATIVE: u16 = 0x0100;
pub const ACC_ABSTRACT: u16 = 0x400;
pub const ACC_SYNTHETIC: u16 = 0x1000;
pub const ACC_ENUM: u16 = 0x4000;

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Debug, PartialEq)]
pub struct ClassResult {
    pub constant_pool: ConstantPool,
    pub access_flags: u16,
    pub this_class_index: u16,
    pub super_class_index: u16,
    pub interfaces: Vec<u16>,
    pub fields: Vec<FieldItem>,
    pub methods: Vec<FieldItem>,
    pub attributes: Vec<AttributeItem>,
    pub signature: Option<u16>,
    pub code: Option<Code>
}

impl ClassResult {
    pub fn new() -> ClassResult {
        ClassResult {
            constant_pool: ConstantPool::new(),
            access_flags: 0,
            this_class_index: 0,
            super_class_index: 0,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            attributes: Vec::new(),
            signature: None,
            code: None
        }
    }

    pub fn name(&self) -> Result<Rc<String>, ClassReadError> {
        return self.constant_pool.get_class_name(self.this_class_index);
    }

    pub fn get_code(&self, target_method_name: &str, target_descriptor: &str) -> Result<Code, ClassReadError> {
        let debug = false;
        let class_name = try!(self.constant_pool.get_class_name(self.this_class_index));
        let mut method_res: Result<&FieldItem, ClassReadError> = Err(ClassReadError::Parse2(format!("Could not find method {} with descriptor {} in class {}", target_method_name, target_descriptor, class_name)));

        for method in &self.methods {
            let method_name = try!(self.constant_pool.get_str(method.name_index));
            let descriptor = try!(self.constant_pool.get_str(method.descriptor_index));
            debugPrint!(debug, 3, "Checking method {} {}", method_name, descriptor);
            if method_name.as_str() == target_method_name &&
                descriptor.as_str() == target_descriptor {
                method_res = Ok(method);
                break;
            }
        }

        let method = try!(method_res);
        debugPrint!(debug, 3, "Found method");
        if (method.access_flags & ACC_NATIVE) != 0 {
            return Err(ClassReadError::NativeMethod(format!("Method '{}' descriptor '{}' in class '{}'", target_method_name, target_descriptor, class_name)));
        } else {
            let code = try!(method.attributes.iter().filter_map(|x|
                match x {
                    &AttributeItem::Code(ref c) => Some(c),
                    _ => None
                })
                .nth(0).ok_or(ClassReadError::Parse2(String::from("Class method has no code"))));
            return Ok(code.clone());
        }
    }

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

fn read_exception(reader: &mut Read) -> Result<ExceptionItem, ClassReadError> {
    let start_pc = try!(reader.read_u16::<BigEndian>());
    let end_pc = try!(reader.read_u16::<BigEndian>());
    let handler_pc = try!(reader.read_u16::<BigEndian>());
    let catch_type = try!(reader.read_u16::<BigEndian>());
    return Ok(ExceptionItem {start_pc: start_pc, end_pc: end_pc, handler_pc: handler_pc, catch_type: catch_type});
}

fn read_attribute(cp: &ConstantPool, reader: &mut Read) -> Result<AttributeItem, ClassReadError> {
    let name_index = try!(reader.read_u16::<BigEndian>());
    let length = try!(reader.read_u32::<BigEndian>());
    let attribute_name = try!(cp.get_str(name_index));
    match attribute_name.as_str() {
        "ConstantValue" => {
            if length != 2 {
                return Err(ClassReadError::Parse);
            } else {
                let index = try!(reader.read_u16::<BigEndian>());
                debugPrint!(true, 4, "ConstantValue attribute with index of {}", index);
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
            debugPrint!(true, 4, "Code attribute with {}B of code, {} exceptions and {} attributes", code_length, exception_table_length, attributes_count);

            return Ok(AttributeItem::Code(Code {
                max_stack: max_stack, max_locals: max_locals, code: code, exceptions: exceptions, attributes: attributes
            }))
        }
        "Exceptions" => {
            let num_exceptions = try!(reader.read_u16::<BigEndian>());
            let mut indicies = Vec::new();
            for _ in 0..num_exceptions {
                indicies.push(try!(reader.read_u16::<BigEndian>()));
            }
            debugPrint!(true, 4, "Exceptions attribute with {} indicies", num_exceptions);

            return Ok(AttributeItem::Exceptions {indicies: indicies})
        }
        "LineNumberTable" => {
            let mut info = Vec::new();
            try!(reader.take(length as u64).read_to_end(&mut info));
            debugPrint!(true, 4, "Ignoring 'LineNumberTable' attribute");
            return Ok(AttributeItem::Unknown {name_index: name_index, info: info});
        }
        "Signature" => {
            let signature_index = try!(reader.read_u16::<BigEndian>());
            debugPrint!(true, 4, "Signature attribute with index {}", signature_index);
            return Ok(AttributeItem::Signature {index: signature_index});
        }
        _ => {
            let mut info = Vec::new();
            try!(reader.take(length as u64).read_to_end(&mut info));
            debugPrint!(true, 4, "Unknown attribute with name {} data {:?}", attribute_name, info);
            return Ok(AttributeItem::Unknown {name_index: name_index, info: info});
        }
    }
}

fn read_field(cp: &ConstantPool, reader: &mut Read) -> Result<FieldItem, ClassReadError> {
    let mut field = FieldItem::new();
    field.access_flags = try!(reader.read_u16::<BigEndian>());
    field.name_index = try!(reader.read_u16::<BigEndian>());
    field.descriptor_index = try!(reader.read_u16::<BigEndian>());

    debugPrint!(true, 4, "Field with name {} descriptor index {}", try!(cp.get_str(field.name_index)), field.descriptor_index);
    let attributes_count = try!(reader.read_u16::<BigEndian>());
    debugPrint!(true, 4, "Field has {} attributes", attributes_count);
    for _ in 0..attributes_count {
        field.attributes.push(try!(read_attribute(cp, reader)));
    }
    return Ok(field);
}

fn string_from_utf8(buf: &Vec<u8>) -> Result<String, ClassReadError> {
    let mut ret = String::new();
    let mut iter = buf.iter();
    let mut maybe_x;
    while {maybe_x = iter.next(); maybe_x.is_some()} {
        let x = *maybe_x.unwrap() as u32;
        if x < 128 {
            ret.push((x as u8) as char);
        } else if x & 0xE0 == 0xC0 {
            let y = *iter.next().unwrap() as u32;
            ret.push(try!(char::from_u32((y & 0x3F) | (x & 0x1F) << 6).ok_or(ClassReadError::UTF8Error(format!("Invalid 2 byte code {} {}", x, y)))));
        } else if x & 0xF0 == 0xE0 {
            let y = *iter.next().unwrap() as u32;
            let z = *iter.next().unwrap() as u32;
            ret.push(try!(char::from_u32((z & 0x3F) | (y & 0x3F) << 6 | (x & 0xF) << 12).ok_or(ClassReadError::UTF8Error(format!("Invalid 3 byte code {} {} {}", x, y, z)))));
        } else if x == 0xED {
            let v = *iter.next().unwrap() as u32;
            let w = *iter.next().unwrap() as u32;
            let head2 = *iter.next().unwrap() as u32;
            if head2 != 0xED {
                return Err(ClassReadError::UTF8Error(format!("Invalid second header byte in 2x3B UTF string {}", head2)));
            }
            let y = *iter.next().unwrap() as u32;
            let z = *iter.next().unwrap() as u32;
            ret.push(try!(char::from_u32((z & 0x3F) | (y & 0xF) << 6 | (w & 0x3F) << 10 | (v & 0xF) << 16 | 0x10000)
                .ok_or(ClassReadError::UTF8Error(format!("Invalid 2x3B code {} {} {} {} {} {}", x, v, w, head2, y, z)))));
        } else {
            return Err(ClassReadError::UTF8Error(format!("Invalid code byte {}", x)));
        }
    }
    return Ok(ret);
}

fn read_constant_pool(reader: &mut Read, entry_count: &mut u16) -> Result<ConstantPoolItem, ClassReadError> {
    let debug = true;
    let tag = try!(reader.read_u8());
    *entry_count = 1;
    match tag {
        1 => {
            // CONSTANT_Utf8
            let length = try!(reader.read_u16::<BigEndian>());
            let mut buf: Vec<u8> = Vec::new();
            try!(reader.take(length as u64).read_to_end(&mut buf));
            let string = try!(string_from_utf8(&buf));
            debugPrint!(debug, 4, "UTF8 {} '{}'", length, string);
            return Ok(ConstantPoolItem::CONSTANT_Utf8(Rc::new(string)));
        },
        3 => {
            // CONSTANT_Integer
            let value = try!(reader.read_u32::<BigEndian>());
            debugPrint!(debug, 4, "Int {}", value);
            return Ok(ConstantPoolItem::CONSTANT_Integer{value: value});
        },
        4 => {
            // CONSTANT_Float
            let value : f32 = unsafe { transmute(try!(reader.read_u32::<BigEndian>())) };
            debugPrint!(debug, 4, "Float {}", value);
            return Ok(ConstantPoolItem::CONSTANT_Float{value: value});
        },
        5 => {
            let value = try!(reader.read_u64::<BigEndian>());
            debugPrint!(debug, 4, "Long {}", value);
            *entry_count = 2;
            return Ok(ConstantPoolItem::CONSTANT_Long{value: value});
        },
        6 => {
            let value : f64 = unsafe { transmute(try!(reader.read_u64::<BigEndian>())) };
            debugPrint!(debug, 4, "Double {}", value);
            *entry_count = 2;
            return Ok(ConstantPoolItem::CONSTANT_Double{value: value});
        },
        7 => {
            // CONSTANT_Class
            let class_index = try!(reader.read_u16::<BigEndian>());
            debugPrint!(debug, 4, "Class ref {}", class_index);
            return Ok(ConstantPoolItem::CONSTANT_Class{index: class_index});
        },
        8 => {
            // CONSTANT_String
            let string_index = try!(reader.read_u16::<BigEndian>());
            debugPrint!(debug, 4, "String ref {}", string_index);
            return Ok(ConstantPoolItem::CONSTANT_String{index:string_index});
        },
        9 => {
            // CONSTANT_Fieldref
            let class_index = try!(reader.read_u16::<BigEndian>());
            let name_and_type_index = try!(reader.read_u16::<BigEndian>());
            debugPrint!(debug, 4, "Field ref {} {}", class_index, name_and_type_index);
            return Ok(ConstantPoolItem::CONSTANT_Fieldref{class_index: class_index, name_and_type_index: name_and_type_index});
        },
        10 => {
            // CONSTANT_Methodref
            let class_index = try!(reader.read_u16::<BigEndian>());
            let name_and_type_index = try!(reader.read_u16::<BigEndian>());
            debugPrint!(debug, 4, "Method ref {} {}", class_index, name_and_type_index);
            return Ok(ConstantPoolItem::CONSTANT_Methodref{class_index: class_index, name_and_type_index: name_and_type_index});
        },
        11 => {
            // CONSTANT_InterfaceMethodref
            let class_index = try!(reader.read_u16::<BigEndian>());
            let name_and_type_index = try!(reader.read_u16::<BigEndian>());
            debugPrint!(debug, 4, "Interface ref {} {}", class_index, name_and_type_index);
            return Ok(ConstantPoolItem::CONSTANT_InterfaceMethodref{class_index: class_index, name_and_type_index: name_and_type_index});
        }
        12 => {
            // CONSTANT_NameAndType
            let name_index = try!(reader.read_u16::<BigEndian>());
            let descriptor_index = try!(reader.read_u16::<BigEndian>());
            debugPrint!(debug, 4, "NameAndType {} {}", name_index, descriptor_index);
            return Ok(ConstantPoolItem::CONSTANT_NameAndType{name_index: name_index, descriptor_index: descriptor_index});
        }
        15 => {
            // CONSTANT_MethodHandle
            let reference_kind = try!(reader.read_u8());
            let reference_index = try!(reader.read_u16::<BigEndian>());
            debugPrint!(debug, 4, "MethodHandle {} {}", reference_kind, reference_index);
            return Ok(ConstantPoolItem::CONSTANT_MethodHandle{reference_kind: reference_kind, reference_index: reference_index});
        }
        16 => {
            // CONSTANT_MethodType
            let descriptor_index = try!(reader.read_u16::<BigEndian>());
            debugPrint!(debug, 4, "MethodType {}", descriptor_index);
            return Ok(ConstantPoolItem::CONSTANT_MethodType{descriptor_index: descriptor_index});
        }
        18 => {
            // CONSTANT_InvokeDynamic
            let bootstrap_method_attr_index = try!(reader.read_u16::<BigEndian>());
            let name_and_type_index = try!(reader.read_u16::<BigEndian>());
            debugPrint!(debug, 4, "InvokeDynamic {} {}", bootstrap_method_attr_index, name_and_type_index);
            return Ok(ConstantPoolItem::CONSTANT_InvokeDynamic{bootstrap_method_attr_index: bootstrap_method_attr_index, name_and_type_index: name_and_type_index});
        }
        _ => {
            debugPrint!(debug, 4, "unknown tag: {}", tag);
            return Err(ClassReadError::Parse);
        }
    }
}

fn read_up_to_my_class_details(filename: &Path) -> Result<(BufReader<File>, ClassResult), ClassReadError> {
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
    debugPrint!(true, 4, "cp: {}", cp_count);

    if cp_count == 0 {
    return Err(ClassReadError::Parse);
    }

    let mut ret = ClassResult::new();

    let mut i = 1;
    while i < cp_count {
        debugPrint!(true, 5, "{}", i);
        let mut entry_count : u16 = 1;
        ret.constant_pool.pool.insert(i, try!(read_constant_pool(&mut reader, &mut entry_count)));
        i += entry_count;
    }

    ret.access_flags = try!(reader.read_u16::<BigEndian>());
    debugPrint!(true, 4, "access_flags: {}", ret.access_flags);
    ret.this_class_index = try!(reader.read_u16::<BigEndian>());
    return Ok((reader, ret));
}

pub fn get_classname(filename: &Path) -> Result<String, ClassReadError> {
    let (_reader, ret) = try!(read_up_to_my_class_details(filename));
    let class_name = try!(ret.constant_pool.get_class_name(ret.this_class_index));
    return Ok((*class_name).clone());
}

pub fn read(filename: &Path) -> Result<ClassResult, ClassReadError> {
    debugPrint!(true, 4, "Reading file {}", filename.display());
    let (mut reader, mut ret) = try!(read_up_to_my_class_details(filename));

    ret.super_class_index = try!(reader.read_u16::<BigEndian>());
    debugPrint!(true, 4, "class_indexes: {} {}", ret.this_class_index, ret.super_class_index);

    let interfaces_count = try!(reader.read_u16::<BigEndian>());
    debugPrint!(true, 4, "Interface count: {}", interfaces_count);
    for _ in 0..interfaces_count {
        ret.interfaces.push(try!(reader.read_u16::<BigEndian>()));
    }

    let fields_count = try!(reader.read_u16::<BigEndian>());
    debugPrint!(true, 4, "Fields count: {}", fields_count);
    for _ in 0..fields_count {
        ret.fields.push(try!(read_field(&ret.constant_pool, &mut reader)));
    }

    let methods_count = try!(reader.read_u16::<BigEndian>());
    debugPrint!(true, 4, "Methods count: {}", methods_count);
    for _ in 0..methods_count {
        ret.methods.push(try!(read_field(&ret.constant_pool, &mut reader)));
    }

    let attributes_count = try!(reader.read_u16::<BigEndian>());
    debugPrint!(true, 4, "Attributes count: {}", attributes_count);
    for _ in 0..attributes_count {
        let attribute = try!(read_attribute(&ret.constant_pool, &mut reader));
        match attribute {
            AttributeItem::Signature{index} => {ret.signature = Some(index);},
            AttributeItem::Code(c) => { ret.code = Some(c); },
            _ => { ret.attributes.push(attribute); }
        }
    }

    return Ok(ret);
}