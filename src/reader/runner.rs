extern crate byteorder;
#[macro_use]
use reader::class::*;
use std;
use std::fmt;
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
    ClassNotLoaded(String),
    NullPointerException,
}

#[derive(Clone, Debug)]
struct Class {
    name: String,
    initialised: bool,
    cr: ClassResult,
    statics: HashMap<String, Variable>
}
impl Class {
  pub fn new(name: &String, cr: &ClassResult) -> Class {
      return Class { name: name.clone(), initialised: false, cr: cr.clone(), statics: HashMap::new()};
  }
}

#[derive(Clone, Debug)]
struct Object {
    typeRef: Rc<Class>,
    members: HashMap<String, Variable>,
}
impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Object type:{}", self.typeRef.name)
    }
}

#[derive(Clone, Debug)]
enum Variable {
    Byte(u8),
    Char(char),
    Double(f64),
    Float(f32),
    Int(i32),
    Long(i64),
    Short(i16),
    Boolean(bool),
    Reference(Rc<Class>, Option<Rc<Object>>),
    ArrayReference(Rc<Variable>, Option<Rc<Vec<Variable>>>), // First argument is dummy for array type
    InterfaceReference(Rc<Object>),
    UnresolvedReference(String),
}
impl fmt::Display for Variable {
     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
         match self {
             &Variable::Reference(ref class, ref maybe_ref) => {
                 write!(f, "Reference ({} {})", class.name, maybe_ref.is_some())
             },
             _ => {
                 write!(f, "{:?}", self)
             }
         }
     }
 }

#[derive(Clone, Debug)]
struct Frame {
    constant_pool: HashMap<u16, ConstantPoolItem>,
    local_variables: Vec<Variable>,
    operand_stack: Vec<Variable>,
}

struct Runtime {
    previous_frames: Vec<Frame>,
    current_frame: Frame,
    class_paths: Vec<String>,
    classes: HashMap<String, Rc<Class>>,
}

fn last_mut(v : &mut Vec<Frame>) -> &mut Frame {
    let len = v.len();
    return &mut v[len-1];
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

fn initialise_variable(classes: &HashMap<String, Rc<Class>>, descriptor_string: &str) -> Result<Variable, RunnerError> {
    let mut variable = try!(parse_single_type_string(classes, descriptor_string));
    return Ok(variable);
}

fn construct_object(classes: &HashMap<String, Rc<Class>>, name: &str, arguments: &Vec<Variable>) -> Result<Variable, RunnerError> {
    debugPrint!(true, 3, "Constructing object {}", name);

    let class = try!(classes.get(name).ok_or(RunnerError::ClassInvalid));
    let mut members : HashMap<String, Variable> = HashMap::new();
    for field in &class.cr.fields {
        if field.access_flags & ACC_STATIC != 0 {
            continue;
        }

        let name_string = try!(get_cp_str(&class.cr.constant_pool, field.name_index));
        let descriptor_string = try!(get_cp_str(&class.cr.constant_pool, field.descriptor_index));

        let var = try!(initialise_variable(classes, descriptor_string));

        members.insert(String::from(name_string), var);
    }
    // TODO: constructor
    let obj = Object {typeRef: class.clone(), members: members};
    return Ok(Variable::Reference(class.clone(), Some(Rc::new(obj))));
}

fn get_class_method_code(class: &ClassResult, method_name: &str, descriptor: &str) -> Result<Code, RunnerError> {
    let mut method_res: Result<&FieldItem, RunnerError> = Err(RunnerError::ClassInvalid);

    for method in &class.methods {
        if try!(get_cp_str(&class.constant_pool, method.name_index)) == method_name &&
            try!(get_cp_str(&class.constant_pool, method.descriptor_index)) == descriptor {
            method_res = Ok(method);
            break;
        }
    }

    let method = try!(method_res);
    let code = try!(method.attributes.iter().filter_map(|x|
        match x {
            &AttributeItem::Code(ref c) => Some(c),
            _ => None
        })
        .nth(0).ok_or(RunnerError::ClassInvalid));
    return Ok(code.clone());
}

fn get_obj_instance_from_variable(var: &Variable) -> Result<Option<Rc<Object>>, RunnerError> {
    match var {
        &Variable::Reference(ref class, ref objref) => {
            return Ok(objref.clone());
        },
        _ => {
            return Err(RunnerError::ClassInvalid);
        }
    }
}

fn construct_char_array(s: &str) -> Variable {
    let mut v : Vec<Variable> = Vec::new();
    for c in s.chars() {
        v.push(Variable::Char(c));
    }
    return Variable::ArrayReference(Rc::new(Variable::Char('\0')), Some(Rc::new(v)));
}

fn run_method(mut runtime: &mut Runtime, code: &Code, pc: u16) -> Result<(), RunnerError> {
    if pc as usize > code.code.len() {
        return Err(RunnerError::InvalidPc);
    }
    let mut buf = Cursor::new(&code.code);

    loop {
        let current_position = buf.position();
        let op_code = try!(buf.read_u8());
        match op_code {
            18 => { // LDC
                let index = try!(buf.read_u8());
                debugPrint!(true, 2, "LDC {}", index);
                let maybe_cp_entry = runtime.current_frame.constant_pool.get(&(index as u16));
                if maybe_cp_entry.is_none() {
                    debugPrint!(true, 1, "LDC failed at index {}", index);
                    return Err(RunnerError::ClassInvalid);
                } else {
                    match *maybe_cp_entry.unwrap() {
                        ConstantPoolItem::CONSTANT_String { index } => {
                            let string_value = try!(get_cp_str(&runtime.current_frame.constant_pool, index));
                            let arguments = vec!(construct_char_array(string_value));
                            let var = try!(construct_object(&mut runtime.classes, &"java/lang/String", &arguments));
                            runtime.current_frame.operand_stack.push(var);
                        }
                        _ => return Err(RunnerError::UnknownOpCode(op_code))
                    }
                }
            },
            42...45 => {
                let index = op_code - 42;
                let loaded = runtime.current_frame.local_variables[index as usize].clone();
                debugPrint!(true, 2, "ALOAD_{} {}", index, loaded);
                runtime.current_frame.operand_stack.push(loaded);
            }
            75...78 => {
                let index = (op_code - 75) as usize;
                let popped = runtime.current_frame.operand_stack.pop().unwrap();
                debugPrint!(true, 2, "ASTORE_{} {}", index, popped);
                let local_len = runtime.current_frame.local_variables.len();
                if local_len > index {
                    runtime.current_frame.local_variables[index as usize] = popped;
                } else if local_len == index {
                    runtime.current_frame.local_variables.push(popped);
                } else {
                    debugPrint!(true, 1, "Asked to store into local variables at index {} when current size is only {}", index, local_len);
                    return Err(RunnerError::InvalidPc);
                }
            }
            89 => {
                let stack_len = runtime.current_frame.operand_stack.len();
                let peek = runtime.current_frame.operand_stack[stack_len - 1].clone();
                debugPrint!(true, 2, "DUP {}", peek);
                runtime.current_frame.operand_stack.push(peek);
            }
            177 => { // return
                debugPrint!(true, 2, "Return");
                return Ok(());
            }
            178 => { // getstatic
                let index = try!(buf.read_u16::<BigEndian>());
                let (class_name, field_name, typ) = try!(load_field(&runtime.current_frame.constant_pool, index));
                debugPrint!(true, 2, "GETSTATIC {} {} {}", class_name, field_name, typ);
                let class_result = try!(load_class(&mut runtime.classes, class_name, &runtime.class_paths));
                let maybe_static_variable = class_result.statics.get(field_name);
                if maybe_static_variable.is_none() {
                    return Err(RunnerError::ClassNotLoaded(String::from(class_name)));
                }
                runtime.current_frame.operand_stack.push(maybe_static_variable.unwrap().clone());
            }
            180 => {
                let field_index = try!(buf.read_u16::<BigEndian>());
                let (class_name, field_name, typ) = try!(load_field(&runtime.current_frame.constant_pool, field_index));
                let var = runtime.current_frame.operand_stack.pop().unwrap();
                let obj = try!(try!(get_obj_instance_from_variable(&var)).ok_or(RunnerError::NullPointerException));
                debugPrint!(true, 2, "GETFIELD {} {} {} {}", class_name, field_name, typ, obj);
                if obj.typeRef.name != class_name {
                    debugPrint!(true, 1, "Getfield called when object on stack had incorrect type");
                    return Err(RunnerError::ClassInvalid);
                }
                let member = try!(obj.members.get(field_name).ok_or(RunnerError::ClassInvalid));
                runtime.current_frame.operand_stack.push(member.clone());
            }
            182 | 183 => {  // invokevirtual, invokespecial
                let mut code : Option<Code> = None;
                let mut new_frame : Option<Frame> = None;
                {
                    let index = try!(buf.read_u16::<BigEndian>());
                    let (class_name, method_name, descriptor) = try!(load_method(&runtime.current_frame.constant_pool, index));
                    debugPrint!(true, 2, "INVOKEVIRTUAL {} {} {}", class_name, method_name, descriptor);
                    let (parameters, return_type) = try!(parse_function_type_string(&runtime.classes, descriptor));
                    let current_op_stack_size = runtime.current_frame.operand_stack.len();
                    let new_local_variables = runtime.current_frame.operand_stack.split_off(current_op_stack_size - parameters.len() - 1);
                    let obj = new_local_variables[0].clone();
                    match obj {
                        Variable::Reference(class, maybe_ref) => {
                            new_frame = Some(Frame {
                                constant_pool: class.cr.constant_pool.clone(),
                                operand_stack: Vec::new(),
                                local_variables: new_local_variables});

                            if class.name != class_name {
                                debugPrint!(true, 1, "Expected object on stack with class name {} but got {}", class_name, class.name);
                                return Err(RunnerError::ClassInvalid);
                            } else if maybe_ref.is_none() {
                                debugPrint!(true, 1, "Expected object on stack with class name {} but got null", class_name);
                                return Err(RunnerError::ClassInvalid);
                            }

                            code = Some(try!(get_class_method_code(&class.cr, method_name, descriptor)));
                        },
                        _ => {
                            debugPrint!(true, 1, "Expected object to invokevirtual on, but got something else {:?}", obj);
                            return Err(RunnerError::ClassInvalid);
                        }
                    }
                }

                runtime.previous_frames.push(runtime.current_frame.clone());
                runtime.current_frame = new_frame.unwrap();
                try!(run_method(&mut runtime, &code.unwrap(), 0));
            },
            194 => {
                let var = runtime.current_frame.operand_stack.pop().unwrap();
                debugPrint!(true, 2, "MONITORENTER {}", var);
                let obj = try!(try!(get_obj_instance_from_variable(&var)).ok_or(RunnerError::NullPointerException));
                // TODO: Implement monitor
                debugPrint!(true, 1, "WARNING: MonitorEnter not implemented");
            },
            199 => {
                let branch_offset = try!(buf.read_u16::<BigEndian>()) as u64;
                let var = runtime.current_frame.operand_stack.pop().unwrap();
                debugPrint!(true, 2, "IFNONNULL {} {}", var, branch_offset);
                let maybe_obj = try!(get_obj_instance_from_variable(&var));
                if maybe_obj.is_some() {
                    debugPrint!(true, 2, "BRANCHED from {} to {}", current_position, current_position + branch_offset);
                    buf.set_position(current_position + branch_offset);
                }
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
    let mut classes_to_process : Vec<Rc<Class>> = Vec::new();

    let new_class = Rc::new(Class::new(&String::from(name), class_result));
    classes.insert(String::from(name), new_class.clone());
    classes_to_process.push(new_class);
    debugPrint!(true, 2, "Finding unresolved dependencies in class {}", name);
    find_unresolved_class_dependencies(classes, &mut unresolved_classes, class_result);

    while unresolved_classes.len() > 0 {
        let class_to_resolve = unresolved_classes.iter().next().unwrap().clone();
        debugPrint!(true, 2, "Finding unresolved dependencies in class {}", class_to_resolve);
        unresolved_classes.remove(&class_to_resolve);
        let class_result_to_resolve = try!(find_class(&class_to_resolve, class_paths));
        let new_class = Rc::new(Class::new(&class_to_resolve, &class_result_to_resolve));
        classes.insert(class_to_resolve, new_class.clone());
        classes_to_process.push(new_class);
        find_unresolved_class_dependencies(classes, &mut unresolved_classes, &class_result_to_resolve);
    }

    for mut class in classes_to_process {
        initialise_class(classes, &class);
    }
    return Ok(classes.get(&String::from(name)).unwrap().clone());
}

fn find_unresolved_class_dependencies(classes: &mut HashMap<String, Rc<Class>>, unresolved_classes: &mut HashSet<String>, class_result: &ClassResult) -> Result<(), RunnerError> {
    let debug = false;
    for field in &class_result.fields {
        let name_string = try!(get_cp_str(&class_result.constant_pool, field.name_index));
        let descriptor_string = try!(get_cp_str(&class_result.constant_pool, field.descriptor_index));

        debugPrint!(debug, 3, "Checking field {} {}", name_string, descriptor_string);

        let variable = try!(parse_single_type_string(classes, descriptor_string));
        match variable {
            Variable::UnresolvedReference(ref type_string) => {
                debugPrint!(debug, 3, "Class {} is unresolved", type_string);
                unresolved_classes.insert(type_string.clone());
            },
            _ => {}
        }
    }
    return Ok(());
}

fn initialise_class(classes: &mut HashMap<String, Rc<Class>>, class: &Rc<Class>) -> Result<(), RunnerError> {
    debugPrint!(true, 2, "Initialising class {}", class.name);
    if class.initialised {
        return Ok(());
    }

    let class_name = class.name.clone();
    let mut class_mut = (**class).clone();
    for field in &class_mut.cr.fields {
        if field.access_flags & ACC_STATIC == 0 {
            continue;
        }

        let name_string = try!(get_cp_str(&class_mut.cr.constant_pool, field.name_index));
        let descriptor_string = try!(get_cp_str(&class_mut.cr.constant_pool, field.descriptor_index));

        debugPrint!(true, 3, "Constructing class static member {} {}", name_string, descriptor_string);

        let var = try!(initialise_variable(classes, descriptor_string));

        class_mut.statics.insert(String::from(name_string), var);
    }
    class_mut.initialised = true;
    classes.insert(String::from(class_name), Rc::new(class_mut));
    return Ok(());
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

    let mut variable = Variable::Int(0);
    match maybe_type_specifier.unwrap() {
        'L' => {
            let type_string : String = iter.take_while(|x| *x != ';').collect();
            if classes.contains_key( type_string.as_str()) {
                let class = classes.get(type_string.as_str()).unwrap().clone();
                variable = Variable::Reference(class.clone(), None);
            } else {
                variable = Variable::UnresolvedReference(type_string.clone());
            }
        }
        'B' => variable = Variable::Byte(0),
        'C' => variable = Variable::Char('\0'),
        'D' => variable = Variable::Double(0.0),
        'F' => variable = Variable::Float(0.0),
        'I' => variable = Variable::Int(0),
        'J' => variable = Variable::Long(0),
        'S' => variable = Variable::Short(0),
        'Z' => variable = Variable::Boolean(false),
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
    iter.next();

    let return_type_string : String = iter.collect();
    if return_type_string == "V" {
        return Ok((parameters, None));
    } else {
        return Ok((parameters, Some(try!(parse_single_type_string(classes, return_type_string.as_str())))));
    }
}

fn construct_field(classes: &HashMap<String, Rc<Class>>, field: &FieldItem, constant_pool: &HashMap<u16, ConstantPoolItem>) -> Result<(Variable, Option<String>), RunnerError> {
    let name_string = try!(get_cp_str(&constant_pool, field.name_index));
    let descriptor_string = try!(get_cp_str(&constant_pool, field.descriptor_index));

    debugPrint!(true, 3, "Constructing field {} {}", name_string, descriptor_string);

    let variable = try!(parse_single_type_string(classes, descriptor_string));
    let unres = match &variable {
        &Variable::UnresolvedReference(ref str) => Some(str.clone()),
        _ => None
      };
    return Ok((variable, unres));
}

pub fn run(class_paths: &Vec<String>, class: &ClassResult) -> Result<(), RunnerError> {
    println!("Running");
    let mut main_method_res : Result<&FieldItem, RunnerError> = Err(RunnerError::ClassInvalid);

    let mut runtime = Runtime {
        class_paths: class_paths.clone(),
        previous_frames: Vec::new(),
        current_frame: Frame {
            constant_pool: class.constant_pool.clone(),
            operand_stack: Vec::new(),
            local_variables: Vec::new()},
        classes: HashMap::new()
    };

    bootstrap_class_and_dependencies(&mut runtime.classes, String::new().as_str(), class, class_paths);

    let main_code = try!(get_class_method_code(class, &"main", &"([Ljava/lang/String;)V"));

    try!(run_method(&mut runtime, &main_code, 0));

    return Ok(());
}