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
    Byte(u32,  u8),
    Char(u32,  char),
    Double(u32,  f64),
    Float(u32,  f32),
    Int(u32,  u32),
    Long(u32,  u64),
    Short(u32,  u16),
    Boolean(u32,  bool),
    Reference(u32,  Rc<Object>),
    UnresolvedReference(u32,  String),
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
                            runtime.operand_stack.push(Variable::Reference(0, Rc::new(Object { typeRef: string_class })));
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
        let mut direct_path = class_path.clone();
        direct_path.push_str(name);
        direct_path.push_str(".class");
        let direct_classname = get_classname(Path::new(&direct_path));
        if direct_classname.is_ok() && direct_classname.unwrap() == name {
            let maybe_read = read(Path::new(&direct_path));
            if maybe_read.is_ok() {
                let maybe_rc = bootstrap_class(classes, name, &maybe_read.unwrap(), class_paths);
                if maybe_rc.is_ok() {
                    return maybe_rc;
                }
            }
        }
        debugPrint!(true, 3, "Finding class {} direct load failed", name);

        // Else try globbing
        let mut glob_path = class_path.clone();
        glob_path.push_str("/**/*.class");
        let maybe_glob = glob(glob_path.as_str());
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

        let maybe_rc = bootstrap_class(classes, name, &maybe_read.unwrap(), class_paths);
        if maybe_rc.is_err() {
            continue;
        }

        return maybe_rc;
    }
    return Err(RunnerError::ClassNotLoaded(String::from(name)));
}

fn bootstrap_class(classes: &mut HashMap<String, Rc<Class>>, name: &str, class_result: &ClassResult, class_paths: &Vec<String>) -> Result<Rc<Class>, RunnerError> {
    let mut class = Class { cr: class_result.clone(), statics: HashMap::new() };
    classes.insert(String::from(name), Rc::new(class.clone()));
    for field in &class.cr.fields {
        let name_string = try!(get_cp_str(&class.cr.constant_pool, field.name_index));
        let descriptor_string = try!(get_cp_str(&class.cr.constant_pool, field.descriptor_index));
        let mut iter = descriptor_string.chars();

        let mut maybe_type_specifier = iter.next();

        if maybe_type_specifier.is_none() {
            debugPrint!(true, 2, "Type specifier blank");
            return Err(RunnerError::ClassInvalid);
        }

        let mut array_depth = 0;
        while maybe_type_specifier.unwrap_or(' ') == '[' {
            array_depth = array_depth + 1;
            maybe_type_specifier = iter.next();
        }

        if maybe_type_specifier.is_none() {
            debugPrint!(true, 2, "Type specifier invalid {}", descriptor_string);
            return Err(RunnerError::ClassInvalid);
        }

        let mut variable = Variable::Int(array_depth, 0);
        match maybe_type_specifier.unwrap() {
            'L' => {
                let type_string : String = iter.take_while(|x| *x != ';').collect();
                debugPrint!(true, 0, "bootstrap static {} {}", name_string, type_string);
                find_class(classes, type_string.as_str(), class_paths);
            }
            'B' => variable = Variable::Byte(array_depth, 0),
            'C' => variable = Variable::Char(array_depth, '\0'),
            'D' => variable = Variable::Double(array_depth, 0.0),
            'F' => variable = Variable::Float(array_depth, 0.0),
            'I' => variable = Variable::Int(array_depth, 0),
            'J' => variable = Variable::Long(array_depth, 0),
            'S' => variable = Variable::Short(array_depth, 0),
            'Z' => variable = Variable::Boolean(array_depth, false),
            _ => {
                debugPrint!(true, 1, "Type string {} for {} unrecognised", descriptor_string, name_string);
                return Err(RunnerError::ClassInvalid);
            }
        }
        class.statics.insert(String::from(name_string), variable);
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

    bootstrap_class(&mut runtime.classes, String::new().as_str(), class, class_paths);

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