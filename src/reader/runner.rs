extern crate byteorder;
#[macro_use]
use reader::class::*;
use std;
use std::io;
use std::io::Cursor;
use std::collections::{HashMap, HashSet};
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
    name: String,
    cr: ClassResult,
    statics: HashMap<String, Variable>
}

#[derive(Clone, Debug)]
struct Object {
    typeRef: Rc<Class>,
    members: HashMap<String, Variable>,
}

#[derive(Clone, Debug)]
enum Variable {
    Byte(u32,  u8),
    Char(u32,  char),
    Double(u32,  f64),
    Float(u32,  f32),
    Int(u32,  i32),
    Long(u32,  i64),
    Short(u32,  i16),
    Boolean(u32,  bool),
    Reference(u32,  Rc<Class>, Option<Rc<Object>>),
    ArrayReference(u32,  Vec<Rc<Object>>),
    InterfaceReference(u32,  Rc<Object>),
    UnresolvedReference(u32,  String),
}

struct Runtime {
    class_paths: Vec<String>,
    constant_pool: HashMap<u16, ConstantPoolItem>,
    operand_stack: Vec<Variable>,
    classes: HashMap<String, Rc<Class>>,
    unresolved_classes: HashSet<String>,
}

impl From<io::Error> for RunnerError {
    fn from(err: io::Error) -> RunnerError {
        RunnerError::ClassInvalid
    }
}

fn get_cp_str(constant_pool: &HashMap<u16, ConstantPoolItem>, index:u16) -> Result<&str, RunnerError> {
    let maybe_cp_entry = constant_pool.get(&index);
    if maybe_cp_entry.is_none() {
        debugPrint!(true, 1, "Missing CP string {}", index);
        return Err(RunnerError::ClassInvalid);
    } else {
        match *maybe_cp_entry.unwrap() {
            ConstantPoolItem::CONSTANT_Utf8(ref s) => {
                return Ok(&s);
            }
            _ => {
                debugPrint!(true, 1, "CP item at index {} is not utf8", index);
                return Err(RunnerError::ClassInvalid);
            }
        }
    }
}

fn load_constpool_class(constant_pool: &HashMap<u16, ConstantPoolItem>, index: u16) -> Result<&str, RunnerError> {
    debugPrint!(false, 5, "{}", index);

    let maybe_cp_entry = constant_pool.get(&index);
    if maybe_cp_entry.is_none() {
        debugPrint!(true, 1, "Missing CP class {}", index);
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
        debugPrint!(true, 1, "Missing CP name & type {}", index);
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
        debugPrint!(true, 1, "Missing CP field {}", index);
        return Err(RunnerError::ClassInvalid);
    } else {
        match *maybe_cp_entry.unwrap() {
            ConstantPoolItem::CONSTANT_Fieldref{class_index, name_and_type_index} => {
                let class_str = try!(load_constpool_class(constant_pool, class_index));
                let (name_str, type_str) = try!(load_name_and_type(constant_pool, name_and_type_index));
                return Ok((class_str, name_str, type_str));
            }
            _ => {
                println!("Index {} is not a field", index);
                return Err(RunnerError::ClassInvalid);
            }
        }
    }
}

fn load_method(constant_pool: &HashMap<u16, ConstantPoolItem>, index: u16) -> Result<(&str, &str, &str), RunnerError> {
    debugPrint!(false, 5, "{}", index);
    let maybe_cp_entry = constant_pool.get(&index);
    if maybe_cp_entry.is_none() {
        debugPrint!(true, 1, "Missing CP method {}", index);
        return Err(RunnerError::ClassInvalid);
    } else {
        match *maybe_cp_entry.unwrap() {
            ConstantPoolItem::CONSTANT_Methodref {class_index, name_and_type_index} => {
                let class_str = try!(load_constpool_class(constant_pool, class_index));
                let (name_str, type_str) = try!(load_name_and_type(constant_pool, name_and_type_index));
                return Ok((class_str, name_str, type_str));
            }
            _ => {
                println!("Index {} is not a method", index);
                return Err(RunnerError::ClassInvalid);
            }
        }
    }
}

fn construct_object(classes: &mut HashMap<String, Rc<Class>>, class: &Rc<Class>) -> Result<Rc<Object>, RunnerError> {
    let mut obj = Object { typeRef: class.clone(), members: HashMap::new()};

    debugPrint!(true, 2, "Constructing object {}", class.name);

    for field in &class.cr.fields {
        if field.access_flags & ACC_STATIC != 0 {
            // Static
            continue;
        }

        let name_string = try!(get_cp_str(&class.cr.constant_pool, field.name_index));
        let (variable, maybe_unres) = try!(construct_field(classes, &field, &class.cr.constant_pool));

        if maybe_unres.is_some() {
            println!("Constructed a nonstatic object field unresolved class {}", maybe_unres.unwrap());
            return Err(RunnerError::ClassInvalid);
        }

        obj.members.insert(String::from(name_string), variable);
    }

    // TODO: Constructor

    return Ok(Rc::new(obj));
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
                debugPrint!(true, 2, "LDC {}", index);
                let maybe_cp_entry = runtime.constant_pool.get(&(index as u16));
                if maybe_cp_entry.is_none() {
                    debugPrint!(true, 1, "LDC failed at index {}", index);
                    return Err(RunnerError::ClassInvalid);
                } else {
                    match *maybe_cp_entry.unwrap() {
                        ConstantPoolItem::CONSTANT_String { index } => {
                            let string_class = try!(load_class(&mut runtime.classes, "java/lang/String", &runtime.class_paths));
                            runtime.operand_stack.push(Variable::Reference(0, string_class.clone(), Some(try!(construct_object(&mut runtime.classes, &string_class)))));
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
                let class_result = try!(load_class(&mut runtime.classes, class_name, &runtime.class_paths));
                let maybe_static_variable = class_result.statics.get(field_name);
                if maybe_static_variable.is_none() {
                    return Err(RunnerError::ClassNotLoaded(String::from(class_name)));
                }
                runtime.operand_stack.push(maybe_static_variable.unwrap().clone());
            }
            182 => {  // invokevirtual
                let index = try!(buf.read_u16::<BigEndian>());
                let (class_name, name, typ) = try!(load_method(&runtime.constant_pool, index));
                debugPrint!(true, 2, "INVOKEVIRTUAL {} {} {}", class_name, name, typ);
                let (parameters, return_type) = try!(parse_function_type_string(&runtime.classes, typ));
            }
            _ => return Err(RunnerError::UnknownOpCode(op_code))
        }
    }
}

fn find_class(name: &str, class_paths: &Vec<String>) -> Result<ClassResult, RunnerError> {
    debugPrint!(true, 4, "Finding class {}", name);
    for class_path in class_paths.iter() {
        let mut direct_path = class_path.clone();
        direct_path.push_str(name);
        direct_path.push_str(".class");
        let direct_classname = get_classname(Path::new(&direct_path));
        if direct_classname.is_ok() && direct_classname.unwrap() == name {
            let maybe_read = read(Path::new(&direct_path));
            if maybe_read.is_ok() {
                return Ok(maybe_read.unwrap());
            }
        }
        debugPrint!(true, 4, "Finding class {} direct load failed", name);

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

        return Ok(maybe_read.unwrap());
    }
    return Err(RunnerError::ClassNotLoaded(String::from(name)));
}
    
fn load_class(classes: &mut HashMap<String, Rc<Class>>, name: &str, class_paths: &Vec<String>) -> Result<Rc<Class>, RunnerError> {
    {
        let maybe_class = classes.get(name);
        if maybe_class.is_some() {
            // Already bootstrapped
            return Ok(maybe_class.unwrap().clone());
        }
    }
    debugPrint!(true, 2, "Finding class {} not already loaded", name);
    let class_result = try!(find_class(name, class_paths));
    let class_obj = try!(bootstrap_class_and_dependencies(classes, name, &class_result, class_paths));

    return Ok(class_obj);
}

fn bootstrap_class_and_dependencies(classes: &mut HashMap<String, Rc<Class>>, name: &str, class_result: &ClassResult, class_paths: &Vec<String>) -> Result<Rc<Class>, RunnerError>  {
    let mut unresolved_classes : HashSet<String> = HashSet::new();
    let class_obj = try!(bootstrap_class(classes, &mut unresolved_classes, name, class_result));
    while unresolved_classes.len() > 0 {
        let class_to_resolve = unresolved_classes.iter().next().unwrap().clone();
        unresolved_classes.remove(&class_to_resolve);
        let class_result_to_resolve = try!(find_class(&class_to_resolve, class_paths));
        try!(bootstrap_class(classes, &mut unresolved_classes, &class_to_resolve, &class_result_to_resolve));
    }
    return Ok(class_obj);
}

fn bootstrap_class(classes: &mut HashMap<String, Rc<Class>>, unresolved_classes: &mut HashSet<String>, class_name: &str, class_result: &ClassResult) -> Result<Rc<Class>, RunnerError> {
    debugPrint!(true, 2, "Bootstrapping class {}", class_name);
    let mut class = Class { name: String::from(class_name), cr: class_result.clone(), statics: HashMap::new() };
    for field in &class.cr.fields {
        let name_string = try!(get_cp_str(&class.cr.constant_pool, field.name_index));
        let (variable, maybe_unres) = try!(construct_field(classes, &field, &class.cr.constant_pool));

        if maybe_unres.is_some() {
            unresolved_classes.insert(maybe_unres.unwrap());
        }

        if field.access_flags & ACC_STATIC != 0 {
            class.statics.insert(String::from(name_string), variable);
        }
    }
    let rc = Rc::new(class);
    debugPrint!(true, 2, "Bootstrap complete {}", class_name);
    classes.insert(String::from(class_name), rc.clone());
    unresolved_classes.remove(class_name);
    return Ok(rc);
}

fn parse_single_type_string(classes: &HashMap<String, Rc<Class>>, string: &str) -> Result<Variable, RunnerError> {
    let mut iter = string.chars();

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
        debugPrint!(true, 2, "Type specifier invalid {}", string);
        return Err(RunnerError::ClassInvalid);
    }

    let mut variable = Variable::Int(array_depth, 0);
    match maybe_type_specifier.unwrap() {
        'L' => {
            let type_string : String = iter.take_while(|x| *x != ';').collect();
            if classes.contains_key( type_string.as_str()) {
                let class = classes.get(type_string.as_str()).unwrap().clone();
                variable = Variable::Reference(array_depth, class.clone(), None);
            } else {
                variable = Variable::UnresolvedReference(array_depth, type_string.clone());
            }
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
            debugPrint!(true, 1, "Type string {} unrecognised", string);
            return Err(RunnerError::ClassInvalid);
        }
    }

    return Ok(variable);
}

fn parse_function_type_string(classes: &HashMap<String, Rc<Class>>, string: &str) -> Result<(Vec<Variable>, Option<Variable>), RunnerError> {
    let debug = true;
    let mut iter = string.chars().peekable();

    if iter.next().unwrap_or(' ') != '(' {
        debugPrint!(debug, 2, "Type {} invalid", string);
        return Err(RunnerError::ClassInvalid);
    }

    let mut parameters : Vec<Variable> = Vec::new();
    while *iter.peek().unwrap_or(&' ') != ')' {
        let single_type_string : String = iter.by_ref().take_while(|x| *x != ';').collect();
        debugPrint!(debug, 3, "Found parameter {}", single_type_string);
        parameters.push(try!(parse_single_type_string(classes, single_type_string.as_str())));
    }

    return Ok((parameters, None));
}

fn construct_field(classes: &mut HashMap<String, Rc<Class>>, field: &FieldItem, constant_pool: &HashMap<u16, ConstantPoolItem>) -> Result<(Variable, Option<String>), RunnerError> {
    let name_string = try!(get_cp_str(&constant_pool, field.name_index));
    let descriptor_string = try!(get_cp_str(&constant_pool, field.descriptor_index));

    debugPrint!(true, 3, "Constructing field {} {}", name_string, descriptor_string);

    let variable = try!(parse_single_type_string(classes, descriptor_string));
    let unres = match &variable {
        &Variable::UnresolvedReference(n, ref str) => Some(str.clone()),
        _ => None
      };
    return Ok((variable, unres));
}

pub fn run(class_paths: &Vec<String>, class: &ClassResult) -> Result<(), RunnerError> {
    println!("Running");
    let mut main_method_res : Result<&FieldItem, RunnerError> = Err(RunnerError::ClassInvalid);

    let mut runtime = Runtime {
        class_paths: class_paths.clone(),
        constant_pool: class.constant_pool.clone(),
        operand_stack: Vec::new(),
        classes: HashMap::new(),
        unresolved_classes: HashSet::new(),
    };

    bootstrap_class_and_dependencies(&mut runtime.classes, String::new().as_str(), class, class_paths);

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