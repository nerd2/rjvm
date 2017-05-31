extern crate byteorder;
#[macro_use]
use reader::class::*;
use std::io;
use std::io::Cursor;
use std::collections::HashMap;
use std::rc::Rc;
use self::byteorder::{BigEndian, ReadBytesExt};

#[derive(Debug)]
pub enum RunnerError {
    ClassInvalid,
    InvalidPc,
    UnknownOpCode(u8),
    ClassNotLoaded(String)
}

struct Class {
    cr: ClassResult,
    statics: HashMap<String, Variable>
}

struct Object {
    typee: Rc<Class>
}

enum Variable {
    Primative(u64),
    Reference(Rc<Object>)
}

struct Runtime {
    constant_pool: Vec<ConstantPoolItem>,
    operand_stack: Vec<Variable>,
    classes: HashMap<String, Class>
}

impl From<io::Error> for RunnerError {
    fn from(err: io::Error) -> RunnerError {
        RunnerError::ClassInvalid
    }
}

fn get_cp_str(constant_pool: &Vec<ConstantPoolItem>, index:u16) -> Result<&str, RunnerError> {
    if index == 0 || index as usize > constant_pool.len() {
        return Err(RunnerError::ClassInvalid);
    } else {
        match constant_pool[(index - 1) as usize] {
            ConstantPoolItem::CONSTANT_Utf8(ref s) => {
                return Ok(&s);
            }
            _ => {
                return Err(RunnerError::ClassInvalid);
            }
        }
    }
}

fn load_class(constant_pool: &Vec<ConstantPoolItem>, index: u16) -> Result<&str, RunnerError> {
    debugPrint!(false, 5, "{}", index);

    if index == 0 || index as usize > constant_pool.len() {
        return Err(RunnerError::ClassInvalid);
    } else {
        match constant_pool[(index - 1) as usize] {
            ConstantPoolItem::CONSTANT_Class {index} => {
                debugPrint!(false, 4, "name_index: {}", index);

                let name_str = try!(get_cp_str(&constant_pool, index));
                return Ok(name_str);
            }
            _ => {
                println!("Index {} is not a class", index);

                return Err(RunnerError::ClassInvalid);
            }
        }
    }
}

fn load_name_and_type(constant_pool: &Vec<ConstantPoolItem>, index: u16) -> Result<(&str, &str), RunnerError> {
    debugPrint!(false, 5, "{}", index);

    if index == 0 || index as usize > constant_pool.len() {
        return Err(RunnerError::ClassInvalid);
    } else {
        match constant_pool[(index - 1) as usize] {
            ConstantPoolItem::CONSTANT_NameAndType {name_index, descriptor_index} => {
                debugPrint!(false, 4, "name_index: {}, descriptor_index: {}", name_index, descriptor_index);

                let name_str = try!(get_cp_str(&constant_pool, name_index));
                let type_str = try!(get_cp_str(&constant_pool, descriptor_index));
                return Ok((name_str, type_str));
            }
            _ => {
                println!("Index {} is not a name and type", index);

                return Err(RunnerError::ClassInvalid);
            }
        }
    }
}

fn load_field(constant_pool: &Vec<ConstantPoolItem>, index: u16) -> Result<(&str, &str, &str), RunnerError> {
    debugPrint!(false, 5, "{}", index);
    if index == 0 || index as usize > constant_pool.len() {
        return Err(RunnerError::ClassInvalid);
    } else {
        match constant_pool[(index - 1) as usize] {
            ConstantPoolItem::CONSTANT_Fieldref{class_index, name_and_type_index} => {
                let class_str = try!(load_class(constant_pool, class_index));
                let (name_str, type_str) = try!(load_name_and_type(constant_pool, name_and_type_index));
                return Ok((class_str, name_str, type_str));
            }
            _ => {
                println!("not field");
                return Err(RunnerError::ClassInvalid);
            }
        }
    }
}

fn load_method(constant_pool: &Vec<ConstantPoolItem>, index: u16) -> Result<(&str, &str, &str), RunnerError> {
    debugPrint!(false, 5, "{}", index);
    if index == 0 || index as usize > constant_pool.len() {
        return Err(RunnerError::ClassInvalid);
    } else {
        match constant_pool[(index - 1) as usize] {
            ConstantPoolItem::CONSTANT_Methodref {class_index, name_and_type_index} => {
                let class_str = try!(load_class(constant_pool, class_index));
                let (name_str, type_str) = try!(load_name_and_type(constant_pool, name_and_type_index));
                return Ok((class_str, name_str, type_str));
            }
            _ => {
                println!("not method");
                return Err(RunnerError::ClassInvalid);
            }
        }
    }
}

fn run_method(jci: &HashMap<String, ClassResult>, runtime: &mut Runtime, code: &Code, pc: u16) -> Result<(), RunnerError> {
    if pc as usize > code.code.len() {
        return Err(RunnerError::InvalidPc);
    }
    let mut buf = Cursor::new(&code.code);

    loop {
        let op_code = try!(buf.read_u8());
        match op_code {
            18 => {
                let index = try!(buf.read_u8());
                match runtime.constant_pool[(index - 1) as usize] {
                    ConstantPoolItem::CONSTANT_String{index} => {
                        //runtime.operand_stack.push(Variable::Reference(Object{typee: Rc::new(Class {c})}));
                        debugPrint!(true, 2, "LDC {}", index);
                    }
                    _ => return Err(RunnerError::UnknownOpCode(op_code))
                }
            }
            177 => { // return
                debugPrint!(true, 2, "Return");
                return Ok(());
            }
            178 => { // getstatic
                let index = try!(buf.read_u16::<BigEndian>());
                let (class_name, field_name, typ) = try!(load_field(&runtime.constant_pool, index));
                debugPrint!(true, 2, "GETSTATIC {} {} {}", class_name, field_name, typ);
                let maybe_class_result = jci.get(class_name);
                if maybe_class_result.is_none() {
                    return Err(RunnerError::ClassNotLoaded(String::from(class_name)));
                }
                bootstrap_class(&mut runtime.classes, class_name, maybe_class_result.unwrap());
                let maybe_static_variable = runtime.classes.get(class_name).unwrap().statics.get(field_name);
                if maybe_static_variable.is_none() {
                    return Err(RunnerError::ClassNotLoaded(String::from(class_name)));
                }
                runtime.operand_stack.push(Variable::Primative(0));
            }
            182 => {  // invokevirtual
                let index = try!(buf.read_u16::<BigEndian>());
                let (class, name, typ) = try!(load_method(&runtime.constant_pool, index));
                debugPrint!(true, 2, "INVOKEVIRTUAL {} {} {}", class, name, typ);
            }
            _ => return Err(RunnerError::UnknownOpCode(op_code))
        }
    }
    return Ok(());
}

fn bootstrap_class(classes: &mut HashMap<String, Class>, name: &str, class_result: &ClassResult) {
    if classes.get(name).is_some() {
        // Already bootstrapped
        return;
    }
    let mut class = Class {cr: class_result.clone(), statics: HashMap::new()};
    for field in &class.cr.fields {
        class.statics.insert(String::from(get_cp_str(&class_result.constant_pool, field.name_index).unwrap()), Variable::Primative(0));
    }
    classes.insert(String::from(name), class);
}

pub fn run(jci: &HashMap<String, ClassResult>, class: &ClassResult) -> Result<(), RunnerError> {
    println!("Running");
    let mut main_method_res : Result<&FieldItem, RunnerError> = Err(RunnerError::ClassInvalid);

    let mut runtime = Runtime {constant_pool: class.constant_pool.to_vec(), operand_stack: Vec::new(), classes: HashMap::new()};

    bootstrap_class(&mut runtime.classes, String::new().as_str(), class);

    for method in &class.methods {
        if try!(get_cp_str(&runtime.constant_pool, method.name_index)) == "main" &&
            try!(get_cp_str(&runtime.constant_pool, method.descriptor_index)) == "([Ljava/lang/String;)V" {
            main_method_res = Ok(method);
            break;
        }
    }

    let main_method = try!(main_method_res);

    let main_code = try!(main_method.attributes.iter().filter_map(|x|
            match x { &AttributeItem::Code(ref c) => Some(c), _ => None })
        .nth(0).ok_or(RunnerError::ClassInvalid));

    try!(run_method(jci, &mut runtime, main_code, 0));

    return Ok(());
}