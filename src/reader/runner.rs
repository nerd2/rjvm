extern crate byteorder;
use reader::class::*;
use std::io;
use std::io::Cursor;
use std::collections::HashMap;
use self::byteorder::{BigEndian, ReadBytesExt};

#[derive(Debug)]
pub enum RunnerError {
    ClassInvalid,
    InvalidPc,
    UnknownOpCode(u8)
}

struct Object {

}

enum Variable {
    Primative(u64),
    Reference(Object)
}

struct Runtime {
    constant_pool: Vec<ConstantPoolItem>,
    operand_stack: Vec<Variable>
}

impl From<io::Error> for RunnerError {
    fn from(err: io::Error) -> RunnerError {
        RunnerError::ClassInvalid
    }
}

fn get_cp_str(runtime: &Runtime, index:u16) -> Result<&str, RunnerError> {
    if index == 0 || index as usize > runtime.constant_pool.len() {
        return Err(RunnerError::ClassInvalid);
    } else {
        match runtime.constant_pool[(index - 1) as usize] {
            ConstantPoolItem::CONSTANT_Utf8(ref s) => {
                return Ok(&s);
            }
            _ => {
                return Err(RunnerError::ClassInvalid);
            }
        }
    }
}

fn load_class(runtime: &Runtime, index: u16) -> Result<&str, RunnerError> {
    println!("{}", index);

    if index == 0 || index as usize > runtime.constant_pool.len() {
        return Err(RunnerError::ClassInvalid);
    } else {
        match runtime.constant_pool[(index - 1) as usize] {
            ConstantPoolItem::CONSTANT_Class {index} => {
                println!("name_index: {}", index);

                let name_str = try!(get_cp_str(runtime, index));
                return Ok(name_str);
            }
            _ => {
                println!("not class");

                return Err(RunnerError::ClassInvalid);
            }
        }
    }
}

fn load_name_and_type(runtime: &Runtime, index: u16) -> Result<(&str, &str), RunnerError> {
    println!("{}", index);

    if index == 0 || index as usize > runtime.constant_pool.len() {
        return Err(RunnerError::ClassInvalid);
    } else {
        match runtime.constant_pool[(index - 1) as usize] {
            ConstantPoolItem::CONSTANT_NameAndType {name_index, descriptor_index} => {
                println!("name_index: {}, descriptor_index: {}", name_index, descriptor_index);

                let name_str = try!(get_cp_str(runtime, name_index));
                let type_str = try!(get_cp_str(runtime, descriptor_index));
                return Ok((name_str, type_str));
            }
            _ => {
                println!("not name and type");

                return Err(RunnerError::ClassInvalid);
            }
        }
    }
}

fn load_field(runtime: &Runtime, index: u16) -> Result<(&str, &str, &str), RunnerError> {
    println!("{}", index);
    if index == 0 || index as usize > runtime.constant_pool.len() {
        return Err(RunnerError::ClassInvalid);
    } else {
        match runtime.constant_pool[(index - 1) as usize] {
            ConstantPoolItem::CONSTANT_Fieldref{class_index, name_and_type_index} => {
                let class_str = try!(load_class(runtime, class_index));
                let (name_str, type_str) = try!(load_name_and_type(runtime, name_and_type_index));
                return Ok((class_str, name_str, type_str));
            }
            _ => {
                println!("not field");
                return Err(RunnerError::ClassInvalid);
            }
        }
    }
}

fn load_method(runtime: &Runtime, index: u16) -> Result<(&str, &str, &str), RunnerError> {
    println!("{}", index);
    if index == 0 || index as usize > runtime.constant_pool.len() {
        return Err(RunnerError::ClassInvalid);
    } else {
        match runtime.constant_pool[(index - 1) as usize] {
            ConstantPoolItem::CONSTANT_Methodref {class_index, name_and_type_index} => {
                let class_str = try!(load_class(runtime, class_index));
                let (name_str, type_str) = try!(load_name_and_type(runtime, name_and_type_index));
                return Ok((class_str, name_str, type_str));
            }
            _ => {
                println!("not method");
                return Err(RunnerError::ClassInvalid);
            }
        }
    }
}

fn run_method(runtime: &mut Runtime, code: &Code, pc: u16) -> Result<(), RunnerError> {
    if pc as usize > code.code.len() {
        return Err(RunnerError::InvalidPc);
    }
    let mut buf = Cursor::new(&code.code);

    while true {
        let opCode = try!(buf.read_u8());
        match opCode {
            18 => { // ldc
                let index = try!(buf.read_u8());
                match runtime.constant_pool[(index - 1) as usize] {
                    ConstantPoolItem::CONSTANT_String{index} => {
                        runtime.operand_stack.push(Variable::Reference(Object{}));
                    }
                    _ => return Err(RunnerError::UnknownOpCode(opCode))
                }
                println!("{}", index);
            }
            177 => { // return
                return Ok(());
            }
            178 => { // getstatic
                let index = try!(buf.read_u16::<BigEndian>());
                let (class, name, typ) = try!(load_field(runtime, index));
                println!("{} {} {}", class, name, typ);
            }
            182 => {  // invokevirtual
                let index = try!(buf.read_u16::<BigEndian>());
                let (class, name, typ) = try!(load_method(runtime, index));
                println!("{} {} {}", class, name, typ);
            }
            _ => return Err(RunnerError::UnknownOpCode(opCode))
        }
    }
    return Ok(());
}

pub fn run(jci: &HashMap<String, ClassResult>, class: &ClassResult) -> Result<(), RunnerError> {
    println!("Running");
    let mut main_method_res : Result<&FieldItem, RunnerError> = Err(RunnerError::ClassInvalid);

    let mut runtime = Runtime {constant_pool: class.constant_pool.to_vec(), operand_stack: Vec::new()};

    for method in &class.methods {
        if try!(get_cp_str(&runtime, method.name_index)) == "main" &&
            try!(get_cp_str(&runtime, method.descriptor_index)) == "([Ljava/lang/String;)V" {
            main_method_res = Ok(method);
            break;
        }
    }

    let main_method = try!(main_method_res);

    let main_code = try!(main_method.attributes.iter().filter_map(|x|
            match x { &AttributeItem::Code(ref c) => Some(c), _ => None })
        .nth(0).ok_or(RunnerError::ClassInvalid));

    try!(run_method(&mut runtime, main_code, 0));

    return Ok(());
}