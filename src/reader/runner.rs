extern crate byteorder;
#[macro_use]
use reader::class::*;
use std::io;
use std::io::Cursor;
use std::collections::HashMap;
use std::rc::Rc;
use std::path::{Path, PathBuf};
use glob::glob;

use self::byteorder::{BigEndian, ReadBytesExt};

#[derive(Debug)]
pub enum RunnerError {
    ClassInvalid,
    InvalidPc,
    UnknownOpCode(u8),
    ClassNotLoaded(String)
}

#[derive(Clone, Debug)]
struct Class {
    cr: ClassResult,
    statics: HashMap<String, Variable>
}

#[derive(Clone, Debug)]
struct Object {
    typeRef: Rc<Class>
}

#[derive(Clone, Debug)]
enum Variable {
    Byte(u8),
    Char(char),
    Double(f64),
    Float(f32),
    Int(u32),
    Long(u64),
    Short(u16),
    Boolean(bool),
    Reference(Rc<Object>),
    UnresolvedReference(String),
}

struct Runtime {
    class_paths: Vec<String>,
    constant_pool: HashMap<u16, ConstantPoolItem>,
    operand_stack: Vec<Variable>,
    classes: HashMap<String, Rc<Class>>
}

impl From<io::Error> for RunnerError {
    fn from(err: io::Error) -> RunnerError {
        RunnerError::ClassInvalid
    }
}

fn get_cp_str(constant_pool: &HashMap<u16, ConstantPoolItem>, index:u16) -> Result<&str, RunnerError> {
    let maybe_cp_entry = constant_pool.get(&index);
    if maybe_cp_entry.is_none() {
        return Err(RunnerError::ClassInvalid);
    } else {
        match *maybe_cp_entry.unwrap() {
            ConstantPoolItem::CONSTANT_Utf8(ref s) => {
                return Ok(&s);
            }
            _ => {
                return Err(RunnerError::ClassInvalid);
            }
        }
    }
}

fn load_class(constant_pool: &HashMap<u16, ConstantPoolItem>, index: u16) -> Result<&str, RunnerError> {
    debugPrint!(false, 5, "{}", index);

    let maybe_cp_entry = constant_pool.get(&index);
    if maybe_cp_entry.is_none() {
        return Err(RunnerError::ClassInvalid);
    } else {
        match *maybe_cp_entry.unwrap() {
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

fn load_name_and_type(constant_pool: &HashMap<u16, ConstantPoolItem>, index: u16) -> Result<(&str, &str), RunnerError> {
    debugPrint!(false, 5, "{}", index);

    let maybe_cp_entry = constant_pool.get(&index);
    if maybe_cp_entry.is_none() {
        return Err(RunnerError::ClassInvalid);
    } else {
        match *maybe_cp_entry.unwrap() {
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

fn load_field(constant_pool: &HashMap<u16, ConstantPoolItem>, index: u16) -> Result<(&str, &str, &str), RunnerError> {
    debugPrint!(false, 5, "{}", index);
    let maybe_cp_entry = constant_pool.get(&index);
    if maybe_cp_entry.is_none() {
        return Err(RunnerError::ClassInvalid);
    } else {
        match *maybe_cp_entry.unwrap() {
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

fn load_method(constant_pool: &HashMap<u16, ConstantPoolItem>, index: u16) -> Result<(&str, &str, &str), RunnerError> {
    debugPrint!(false, 5, "{}", index);
    let maybe_cp_entry = constant_pool.get(&index);
    if maybe_cp_entry.is_none() {
        return Err(RunnerError::ClassInvalid);
    } else {
        match *maybe_cp_entry.unwrap() {
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

fn run_method(runtime: &mut Runtime, code: &Code, pc: u16) -> Result<(), RunnerError> {
    if pc as usize > code.code.len() {
        return Err(RunnerError::InvalidPc);
    }
    let mut buf = Cursor::new(&code.code);

    loop {
        let op_code = try!(buf.read_u8());
        match op_code {
            18 => { // LDC
                let index = try!(buf.read_u8());
                let maybe_cp_entry = runtime.constant_pool.get(&(index as u16));
                if maybe_cp_entry.is_none() {
                    return Err(RunnerError::ClassInvalid);
                } else {
                    match *maybe_cp_entry.unwrap() {
                        ConstantPoolItem::CONSTANT_String { index } => {
                            let string_class = try!(find_class(&mut runtime.classes, "java/lang/String", &runtime.class_paths));
                            runtime.operand_stack.push(Variable::Reference(Rc::new(Object { typeRef: string_class })));
                            debugPrint!(true, 2, "LDC {}", index);
                        }
                        _ => return Err(RunnerError::UnknownOpCode(op_code))
                    }
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
                let class_result = try!(find_class(&mut runtime.classes, class_name, &runtime.class_paths));
                let maybe_static_variable = runtime.classes.get(class_name).unwrap().statics.get(field_name);
                if maybe_static_variable.is_none() {
                    return Err(RunnerError::ClassNotLoaded(String::from(class_name)));
                }
                runtime.operand_stack.push(maybe_static_variable.unwrap().clone());
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

fn find_class(classes: &mut HashMap<String, Rc<Class>>, name: &str, class_paths: &Vec<String>) -> Result<Rc<Class>, RunnerError> {
    {
        let maybe_class = classes.get(name);
        if maybe_class.is_some() {
            // Already bootstrapped
            return Ok(maybe_class.unwrap().clone());
        }
    }
    debugPrint!(true, 2, "Finding class {} not already loaded", name);
    for class_path in class_paths.iter() {
        let maybe_glob = glob(class_path);
        if maybe_glob.is_err() {
            debugPrint!(true, 1, "Error globbing class path {}", class_path);
            continue;
        }

        let class_match = maybe_glob.unwrap()
            .filter_map(Result::ok)
            .filter(|x| { let classname = get_classname(&x); return classname.is_ok() && classname.unwrap() == name; } )
            .nth(0);

        if class_match.is_none() {
            debugPrint!(true, 2, "Could not find {} on class path {}", name, class_path);
            continue;
        }

        let maybe_read = read(&class_match.unwrap());
        if maybe_read.is_err() {
            debugPrint!(true, 1, "Error reading class {} on class path {}", name, class_path);
            continue;
        }

        let maybe_rc = bootstrap_class(classes, name, &maybe_read.unwrap());
        if maybe_rc.is_err() {
            continue;
        }

        return maybe_rc;
    }
    return Err(RunnerError::ClassNotLoaded(String::from(name)));
}

fn bootstrap_class(classes: &mut HashMap<String, Rc<Class>>, name: &str, class_result: &ClassResult) -> Result<Rc<Class>, RunnerError> {
    let mut class = Class { cr: class_result.clone(), statics: HashMap::new() };
    for field in &class.cr.fields {
        class.statics.insert(String::from(get_cp_str(&class.cr.constant_pool, field.name_index).unwrap()), Variable::Int(0));
    }
    let rc = Rc::new(class);
    classes.insert(String::from(name), rc.clone());
    return Ok(rc);
}

pub fn run(class_paths: &Vec<String>, class: &ClassResult) -> Result<(), RunnerError> {
    println!("Running");
    let mut main_method_res : Result<&FieldItem, RunnerError> = Err(RunnerError::ClassInvalid);

    let mut runtime = Runtime {
        class_paths: class_paths.clone(),
        constant_pool: class.constant_pool.clone(),
        operand_stack: Vec::new(),
        classes: HashMap::new(),
    };

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

    try!(run_method(&mut runtime, main_code, 0));

    return Ok(());
}