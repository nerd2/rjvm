#![deny(
    non_snake_case,
    unreachable_code,
    unused_assignments,
    unused_imports,
    unused_variables,
    unused_mut,
)]

extern crate byteorder;
extern crate rand;
use reader::builtins::*;
use reader::class::*;
use reader::util::*;
use std;
use std::collections::HashMap;
use std::cell::RefCell;
use std::fmt;
use std::io;
use std::io::Cursor;
use std::ops::BitAnd;
use std::ops::BitOr;
use std::ops::BitXor;
use std::rc::Rc;
use std::rc::Weak;
use std::path::Path;
use std::path::PathBuf;
use glob::glob;

use self::byteorder::{BigEndian, ReadBytesExt};

macro_rules! runnerPrint {
    ($runtime:expr, $enabled:expr, $level:expr, $fmt:expr) => {{if $enabled && $level <= PRINT_LEVEL!() { for _ in 1..$runtime.previous_frames.len() {print!("|"); } print!("{}: ", $runtime.count); println!($fmt); } }};
    ($runtime:expr, $enabled:expr, $level:expr, $fmt:expr, $($arg:tt)*) => {{if $enabled && $level <= PRINT_LEVEL!() { for _ in 1..$runtime.previous_frames.len() {print!("|"); } print!("{}: ", $runtime.count); println!($fmt, $($arg)*); } }};
}

#[derive(Debug)]
pub enum RunnerError {
    ClassInvalid(&'static str),
    ClassInvalid2(String),
    InvalidPc,
    IoError,
    NativeMethod(String),
    UnknownOpCode(u8),
    ClassNotLoaded(String),
    Exception(Variable),
    Return,
    Invoke
}

#[derive(Clone, Debug, PartialEq)]
pub struct Class {
    pub name: String,
    pub cr: ClassResult,
    pub initialising: RefCell<bool>,
    pub initialised: RefCell<bool>,
    pub statics: RefCell<HashMap<String, Variable>>,
    pub super_class: RefCell<Option<Rc<Class>>>
}
impl Class {
  pub fn new(name: &String, cr: &ClassResult) -> Class {
      return Class { name: name.clone(), initialising: RefCell::new(false), initialised: RefCell::new(false), cr: cr.clone(), statics: RefCell::new(HashMap::new()), super_class: RefCell::new(None)};
  }
}

#[derive(Clone, Debug)]
pub struct Object {
    pub is_null: bool,
    pub type_ref: Rc<Class>,
    pub members: RefCell<HashMap<String, Variable>>,
    pub super_class: RefCell<Option<Rc<Object>>>,
    pub sub_class: RefCell<Option<Weak<Object>>>,
    pub code: i32
}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return match self.type_ref.name.as_str() {
            "java/lang/String" => {
                let str = string_to_string(self);
                write!(f, "String {} '{}' null:{}", self.code, str.as_str(), self.is_null)
            }
            _ => {write!(f, "Object {} type:{} null:{}",self.code, self.type_ref.name.as_str(), self.is_null) }
        }
    }
}
impl PartialEq for Object { // Have to implement PartialEq because not derrivable for Weaks in general. We can assume the weak ref is constant.
    fn eq(&self, other: &Self) -> bool {
        let self_sub_class = self.sub_class.borrow();
        let other_sub_class = other.sub_class.borrow();

        return self.type_ref == other.type_ref &&
            self.members == other.members &&
            self_sub_class.is_some() == other_sub_class.is_some() &&
            (self_sub_class.is_none() || (self_sub_class.clone().unwrap().upgrade() == other_sub_class.clone().unwrap().upgrade())) &&
            self.super_class == other.super_class;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArrayObject { // Can be either typed or primitive array (including nested)
    pub is_null: bool,
    pub element_type_ref: Option<Rc<Class>>,
    pub element_type_str: String,
    pub elements: RefCell<Vec<Variable>>,
    pub code: i32,
}

impl fmt::Display for ArrayObject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_null {
            write!(f, "Array of type {} is NULL", self.element_type_str)
        } else {
            let vec = self.elements.borrow();
            write!(f, "Array of type {} Size:{} ({})",
                   self.element_type_str,
                   vec.len(),
                   vec.iter()
                       .take(10)
                       .map(|y| format!("{}", y))
                       .fold(String::new(), |a, b| (a + ", " + b.as_str())))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Variable {
    Byte(u8),
    Char(char),
    Double(f64),
    Float(f32),
    Int(i32),
    Long(i64),
    Short(i16),
    Boolean(bool),
    Reference(Rc<Object>),
    ArrayReference(Rc<ArrayObject>),
    InterfaceReference(Rc<Object>),
    UnresolvedReference(String),
}

impl Variable {
    pub fn to_bool(&self) -> bool {
        match self {
            &Variable::Boolean(ref x) => {
                return *x;
            },
            &Variable::Int(ref x) => {
                return *x != 0;
            },
            _ => {
                panic!("Couldn't convert to bool");
            }
        }
    }
    pub fn to_char(&self) -> char {
        match self {
            &Variable::Char(ref x) => {
                return *x;
            },
            _ => {
                panic!("Couldn't convert to char");
            }
        }
    }
    pub fn to_int(&self) -> i32 {
        match self {
            &Variable::Boolean(ref x) => {
                return if *x { 1 } else { 0 };
            },
            &Variable::Char(ref x) => {
                return *x as i32;
            },
            &Variable::Byte(ref x) => {
                return *x as i32;
            },
            &Variable::Short(ref x) => {
                return *x as i32;
            },
            &Variable::Int(ref x) => {
                return *x;
            },
            _ => {
                panic!("Couldn't convert to int");
            }
        }
    }

    pub fn to_long(&self) -> i64 {
        match self {
            &Variable::Long(ref x) => {
                return *x;
            },
            _ => {
                panic!("Couldn't convert to long");
            }
        }
    }
    pub fn to_float(&self) -> f32 {
        match self {
            &Variable::Float(ref x) => {
                return *x;
            },
            _ => {
                panic!("Couldn't convert to float");
            }
        }
    }
    pub fn to_double(&self) -> f64 {
        match self {
            &Variable::Double(ref x) => {
                return *x;
            },
            _ => {
                panic!("Couldn't convert to double");
            }
        }
    }
    pub fn to_ref_type(&self) -> Rc<Class> {
        match self {
            &Variable::Reference(ref obj) => {
                return obj.type_ref.clone();
            },
            _ => {
                panic!("Couldn't convert to reference");
            }
        }
    }
    pub fn to_ref(&self) -> Rc<Object> {
        match self {
            &Variable::Reference(ref obj) => {
                return obj.clone();
            },
            _ => {
                panic!("Couldn't convert '{}' to reference", self);
            }
        }
    }
    pub fn is_ref_or_array(&self) -> bool {
        match self {
            &Variable::Reference(ref _obj) => {
                return true;
            },
            &Variable::ArrayReference(ref _array) => {
                return true;
            },
            _ => {
                panic!("Couldn't convert '{}' to reference or array", self);
            }
        }
    }
    pub fn is_null(&self) -> bool {
        match self {
            &Variable::Reference(ref obj) => {
                return obj.is_null;
            },
            &Variable::ArrayReference(ref array) => {
                return array.is_null;
            },
            &Variable::UnresolvedReference(ref _x) => {
                return true;
            },
            _ => {
                panic!("Couldn't check if primitive '{}' is null", self);
            }
        }
    }
    pub fn to_arrayobj(&self) -> Rc<ArrayObject> {
        match self {
            &Variable::ArrayReference(ref array) => {
                return array.clone();
            },
            _ => {
                panic!("Couldn't convert to reference");
            }
        }
    }
    pub fn is_type_1(&self) -> bool {
        match self {
            &Variable::Long(_x) => {
                return false;
            },
            &Variable::Double(_y) => {
                return false;
            },
            _ => {
                return true;
            }
        }
    }
    pub fn can_convert_to_int(&self) -> bool {
        return match self {
            &Variable::Boolean(_x) => true,
            &Variable::Byte(_x) => true,
            &Variable::Short(_x) => true,
            &Variable::Char(_x) => true,
            &Variable::Int(_x) => true,
            _ => false,
        }
    }
    pub fn is_primitive(&self) -> bool {
        return match self {
            &Variable::Reference(ref _x) => false,
            &Variable::ArrayReference(ref _x) => false,
            &Variable::InterfaceReference(ref _x) => false,
            &Variable::UnresolvedReference(ref _x) => false,
            _ => true,
        }
    }

    pub fn is_unresolved(&self) -> bool {
        return match self {
            &Variable::UnresolvedReference(ref _x) => true,
            _ => false,
        }
    }

    pub fn get_unresolved_type_name(&self) -> String {
        return match self {
            &Variable::UnresolvedReference(ref type_name) => type_name.clone(),
            _ => panic!("Cannot get unresolved type name of {}", self),
        }
    }

    pub fn hash_code(&self, runtime: &mut Runtime) -> Result<i32, RunnerError> {
        match self {
                &Variable::Reference(ref obj) => {
                    if obj.is_null {
                        let exception = try!(construct_object(runtime, &"java/lang/NullPointerException"));
                        return Err(RunnerError::Exception(exception));
                    } else {
                        return Ok(obj.code);
                    }
                },
                &Variable::ArrayReference(ref obj) => {
                    if obj.is_null {
                        let exception = try!(construct_object(runtime, &"java/lang/NullPointerException"));
                        return Err(RunnerError::Exception(exception));
                    } else {
                        return Ok(obj.code);
                    }
                },
                _ => {
                    panic!("Called hashcode on primitive type");
                }
            };
    }

    pub fn get_descriptor(&self) -> String {
        let mut ret = String::new();
        match self {
            &Variable::Byte(_v) => {ret.push('B');},
            &Variable::Char(_v) => {ret.push('C');},
            &Variable::Double(_v) => {ret.push('D');},
            &Variable::Float(_v) => {ret.push('F');},
            &Variable::Int(_v) => {ret.push('I');},
            &Variable::Long(_v) => {ret.push('J');},
            &Variable::Short(_v) => {ret.push('S');},
            &Variable::Boolean(_v) => {ret.push('Z');},
            &Variable::Reference(ref obj) => {return generate_class_descriptor(&obj.type_ref); },
            &Variable::ArrayReference(ref array_obj) => {
                ret.push('[');
                if array_obj.element_type_ref.is_some() {
                    ret.push_str(generate_class_descriptor(array_obj.element_type_ref.as_ref().unwrap()).as_str());
                } else {
                    ret.push_str(array_obj.element_type_str.as_str());
                }
            },
            &Variable::UnresolvedReference(ref class_name) => {
                ret.push('L');
                ret.push_str(class_name.as_str());
                ret.push(';');
            },
            _ => {panic!("Type not covered");}
        }
        return ret;
    }

    pub fn display(&self) -> String {
        return match self {
            &Variable::Reference(ref obj) => format!("Reference {}", obj),
            &Variable::ArrayReference(ref array) => format!("ArrayReference {}", array),
            _ => format!("{:?}", self)
        }
    }

    pub fn extract_string(&self) -> String {
        match self {
            &Variable::Reference(ref obj) => {
                match obj.type_ref.name.as_str() {
                    "java/lang/String" => {
                        return string_to_string(obj);
                    },
                    _ => {panic!("{} is not a string", self);}
                }
            }
            _ => {panic!("{} is not a string", self);}
        }
    }
}
impl fmt::Display for Variable {
     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
         return write!(f, "{}", self.display());
     }
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub class: Option<Rc<Class>>,
    pub constant_pool: HashMap<u16, ConstantPoolItem>,
    pub local_variables: Vec<Variable>,
    pub operand_stack: Vec<Variable>,
    pub return_pos: u64,
    pub code: Code,
    pub name: String
}
impl Frame {
    pub fn new() -> Frame {
        Frame {
            class: None,
            constant_pool: HashMap::new(),
            operand_stack: Vec::new(),
            local_variables: Vec::new(),
            return_pos: 0,
            code: Code::new(),
            name: String::new()}
    }
}

pub struct Runtime {
    pub previous_frames: Vec<Frame>,
    pub current_frame: Frame,
    pub class_paths: Vec<String>,
    pub classes: HashMap<String, Rc<Class>>,
    pub count: i64,
    pub current_thread: Option<Variable>,
    pub string_interns: HashMap<String, Variable>,
    pub properties: HashMap<String, Variable>,
    pub class_objects: HashMap<String, Variable>,
    pub object_count: i32,
}
impl Runtime {
    fn  new(class_paths: Vec<String>) -> Runtime {
        return Runtime {
            class_paths: class_paths,
            previous_frames: vec!(Frame::new()),
            current_frame: Frame::new(),
            classes: HashMap::new(),
            count: 0,
            current_thread: None,
            string_interns: HashMap::new(),
            properties: HashMap::new(),
            class_objects: HashMap::new(),
            object_count: rand::random::<i32>(),
        };
    }

    pub fn reset_frames(&mut self) {
        self.previous_frames = vec!(Frame::new());
        self.current_frame = Frame::new();
    }

    pub fn get_next_object_code(&mut self) -> i32 {
        let ret = self.object_count;
        self.object_count += 1;
        return ret;
    }

    pub fn push_on_stack(&mut self, var: Variable) {
        if !var.is_type_1() {
            self.current_frame.operand_stack.push(var.clone());
        }
        self.current_frame.operand_stack.push(var);
    }
}

impl From<io::Error> for RunnerError {
    fn from(_err: io::Error) -> RunnerError {
        RunnerError::IoError
    }
}
impl From<ClassReadError> for RunnerError {
    fn from(err: ClassReadError) -> RunnerError {
        RunnerError::ClassInvalid2(format!("{:?}", err))
    }
}

fn descriptor_to_type_name(string: &str) -> Result<String, RunnerError> {
    let mut iter = string.chars();

    let mut maybe_type_specifier = iter.next();

    if maybe_type_specifier.is_none() {
        return Err(RunnerError::ClassInvalid("Type specifier blank"));
    }

    let mut array_depth = 0;
    while maybe_type_specifier.unwrap_or(' ') == '[' {
        array_depth = array_depth + 1;
        maybe_type_specifier = iter.next();
    }

    if maybe_type_specifier.is_none() {
        return Err(RunnerError::ClassInvalid2(format!("Type specifier invalid {}", string)));
    }

    let mut ret : String =
        match maybe_type_specifier.unwrap() {
            'L' => iter.take_while(|x| *x != ';').collect(),
            _ => {
                String::from(match maybe_type_specifier.unwrap() {
                    'B' => "byte",
                    'C' => "char",
                    'D' => "double",
                    'F' => "float",
                    'I' => "int",
                    'J' => "long",
                    'S' => "short",
                    'Z' => "boolean",
                    _ => return Err(RunnerError::ClassInvalid2(format!("Type specifier invalid {}", string)))
                })
            }
        };

    while array_depth > 0 {
        ret.push_str("[]");
        array_depth = array_depth - 1;
    }

    return Ok(ret);
}

pub fn get_cp_str(constant_pool: &HashMap<u16, ConstantPoolItem>, index:u16) -> Result<Rc<String>, RunnerError> {
    let maybe_cp_entry = constant_pool.get(&index);
    if maybe_cp_entry.is_none() {
        debugPrint!(true, 1, "Missing CP string {}", index);
        return Err(RunnerError::ClassInvalid2(format!("Missing CP string {}", index)));
    } else {
        match *maybe_cp_entry.unwrap() {
            ConstantPoolItem::CONSTANT_Utf8(ref s) => {
                return Ok(s.clone());
            }
            _ => {
                debugPrint!(true, 1, "CP item at index {} is not utf8", index);
                return Err(RunnerError::ClassInvalid2(format!("CP item at index {} is not utf8", index)));
            }
        }
    }
}

fn pop_from_stack(operand_stack: &mut Vec<Variable>) -> Option<Variable> {
    let maybe_var = operand_stack.pop();
    maybe_var.as_ref().map(|x| {if !x.is_type_1() {operand_stack.pop();}});
    return maybe_var;
}

fn get_cp_class(constant_pool: &HashMap<u16, ConstantPoolItem>, index: u16) -> Result<Rc<String>, RunnerError> {
    debugPrint!(false, 5, "{}", index);

    let maybe_cp_entry = constant_pool.get(&index);
    if maybe_cp_entry.is_none() {
        debugPrint!(true, 1, "Missing CP class {}", index);
        return Err(RunnerError::ClassInvalid2(format!("Missing CP class {}", index)));
    } else {
        match *maybe_cp_entry.unwrap() {
            ConstantPoolItem::CONSTANT_Class {index} => {
                debugPrint!(false, 4, "name_index: {}", index);

                let name_str = try!(get_cp_str(&constant_pool, index));
                return Ok(name_str);
            }
            _ => {
                return Err(RunnerError::ClassInvalid2(format!("Index {} is not a class", index)));
            }
        }
    }
}

fn get_cp_name_and_type(constant_pool: &HashMap<u16, ConstantPoolItem>, index: u16) -> Result<(Rc<String>, Rc<String>), RunnerError> {
    debugPrint!(false, 5, "{}", index);

    let maybe_cp_entry = constant_pool.get(&index);
    if maybe_cp_entry.is_none() {
        debugPrint!(true, 1, "Missing CP name & type {}", index);
        return Err(RunnerError::ClassInvalid2(format!("Missing CP name & type {}", index)));
    } else {
        match *maybe_cp_entry.unwrap() {
            ConstantPoolItem::CONSTANT_NameAndType {name_index, descriptor_index} => {
                debugPrint!(false, 4, "name_index: {}, descriptor_index: {}", name_index, descriptor_index);

                let name_str = try!(get_cp_str(&constant_pool, name_index));
                let type_str = try!(get_cp_str(&constant_pool, descriptor_index));
                return Ok((name_str, type_str));
            }
            _ => {
                return Err(RunnerError::ClassInvalid2(format!("Index {} is not a name and type", index)));
            }
        }
    }
}

fn get_cp_field(constant_pool: &HashMap<u16, ConstantPoolItem>, index: u16) -> Result<(Rc<String>, Rc<String>, Rc<String>), RunnerError> {
    debugPrint!(false, 5, "{}", index);
    let maybe_cp_entry = constant_pool.get(&index);
    if maybe_cp_entry.is_none() {
        return Err(RunnerError::ClassInvalid2(format!( "Missing CP field {}", index)));
    } else {
        match *maybe_cp_entry.unwrap() {
            ConstantPoolItem::CONSTANT_Fieldref{class_index, name_and_type_index} => {
                let class_str = try!(get_cp_class(constant_pool, class_index));
                let (name_str, type_str) = try!(get_cp_name_and_type(constant_pool, name_and_type_index));
                return Ok((class_str, name_str, type_str));
            }
            _ => {
                return Err(RunnerError::ClassInvalid2(format!("Index {} is not a field {:?}", index, *maybe_cp_entry.unwrap())));
            }
        }
    }
}

fn get_cp_method(constant_pool: &HashMap<u16, ConstantPoolItem>, index: u16) -> Result<(Rc<String>, Rc<String>, Rc<String>), RunnerError> {
    debugPrint!(false, 5, "{}", index);
    let maybe_cp_entry = constant_pool.get(&index);
    if maybe_cp_entry.is_none() {
        debugPrint!(true, 1, "Missing CP method {}", index);
        return Err(RunnerError::ClassInvalid2(format!("Missing CP method {}", index)));
    } else {
        match *maybe_cp_entry.unwrap() {
            ConstantPoolItem::CONSTANT_Methodref {class_index, name_and_type_index} => {
                let class_str = try!(get_cp_class(constant_pool, class_index));
                let (name_str, type_str) = try!(get_cp_name_and_type(constant_pool, name_and_type_index));
                return Ok((class_str, name_str, type_str));
            }
            ConstantPoolItem::CONSTANT_InterfaceMethodref {class_index, name_and_type_index} => {
                let class_str = try!(get_cp_class(constant_pool, class_index));
                let (name_str, type_str) = try!(get_cp_name_and_type(constant_pool, name_and_type_index));
                return Ok((class_str, name_str, type_str));
            }
            _ => {
                return Err(RunnerError::ClassInvalid2(format!("Index {} is not a method", index)));
            }
        }
    }
}

fn get_most_sub_class(mut obj: Rc<Object>) -> Rc<Object>{
    // Go to top of chain
    while obj.sub_class.borrow().is_some() {
        let new_obj = obj.sub_class.borrow().as_ref().unwrap().upgrade().unwrap();
        obj = new_obj;
    }
    return obj;
}

fn initialise_variable(runtime: &mut Runtime, descriptor_string: &str) -> Result<Variable, RunnerError> {
    let variable = try!(parse_single_type_string(runtime, descriptor_string, false));
    return Ok(variable);
}

pub fn construct_char_array(runtime: &mut Runtime, s: &str) -> Variable {
    let mut v : Vec<Variable> = Vec::new();
    for c in s.chars() {
        v.push(Variable::Char(c));
    }
    let array_object = ArrayObject {
        is_null: false,
        element_type_ref: None,
        element_type_str: String::from("C"),
        elements: RefCell::new(v),
        code: runtime.get_next_object_code()
    };
    return Variable::ArrayReference(Rc::new(array_object));
}

pub fn construct_array(runtime: &mut Runtime, class: Rc<Class>, data: Option<Vec<Variable>>) -> Result<Variable, RunnerError> {
    let array_object = ArrayObject {
        is_null: data.is_none(),
        element_type_ref: Some(class.clone()),
        element_type_str: generate_class_descriptor(&class),
        elements: RefCell::new(data.unwrap_or(Vec::new())),
        code: runtime.get_next_object_code()
    };
    return Ok(Variable::ArrayReference(Rc::new(array_object)));
}

pub fn construct_array_by_name(runtime: &mut Runtime, name: &str, data: Option<Vec<Variable>>) -> Result<Variable, RunnerError> {
    let class = try!(load_class(runtime, name));
    return construct_array(runtime, class, data);
}

fn construct_primitive_array(runtime: &mut Runtime, element_type: &str, data: Option<Vec<Variable>>) -> Result<Variable, RunnerError> {
    // TODO
    let array_object = ArrayObject {
        is_null: data.is_none(),
        element_type_ref: None,
        element_type_str: String::from(element_type),
        elements: RefCell::new(data.unwrap_or(Vec::new())),
        code: runtime.get_next_object_code()
    };
    return Ok(Variable::ArrayReference(Rc::new(array_object)));
}

fn construct_null_object(runtime: &mut Runtime, class: Rc<Class>) -> Result<Variable, RunnerError> {
    let obj = Rc::new(Object {
        is_null: true,
        type_ref: class,
        members: RefCell::new(HashMap::new()),
        super_class: RefCell::new(None),
        sub_class: RefCell::new(None),
        code: runtime.get_next_object_code()
    });
    return Ok(Variable::Reference(obj));
}

pub fn construct_null_object_by_name(runtime: &mut Runtime, name: &str) -> Result<Variable, RunnerError> {
    return parse_single_type_string(runtime, name, true);
}

pub fn construct_object(runtime: &mut Runtime, name: &str) -> Result<Variable, RunnerError> {
    let debug = false;
    runnerPrint!(runtime, debug, 3, "Constructing object {}", name);
    try!(load_class(runtime, name));

    let original_class = try!(load_class(runtime, name));
    let mut original_obj : Option<Rc<Object>> = None;
    let mut class = original_class.clone();
    let mut sub_class : Option<Weak<Object>> = None;

    loop {
        runnerPrint!(runtime, debug, 3, "Constructing object of type {} with subclass {}", class.name, sub_class.is_some());
        let mut members: HashMap<String, Variable> = HashMap::new();
        for field in &class.cr.fields {
            if field.access_flags & ACC_STATIC != 0 {
                continue;
            }

            let name_string = try!(get_cp_str(&class.cr.constant_pool, field.name_index));
            let descriptor_string = try!(get_cp_str(&class.cr.constant_pool, field.descriptor_index));

            let var = try!(initialise_variable(runtime, descriptor_string.as_str()));

            members.insert((*name_string).clone(), var);
        }

        let obj = Rc::new(Object {
            is_null: false,
            type_ref: class.clone(),
            members: RefCell::new(members),
            super_class: RefCell::new(None),
            sub_class: RefCell::new(sub_class.clone()),
            code: runtime.get_next_object_code()
        });
        if original_obj.is_none() {
            original_obj = Some(obj.clone());
        }
        if sub_class.is_some() {
            let sub_class_up = sub_class.unwrap().upgrade().unwrap();
            *sub_class_up.super_class.borrow_mut() = Some(obj.clone());
        }
        let maybe_super_class = class.super_class.borrow().clone();
        if maybe_super_class.is_some() {
            sub_class = Some(Rc::downgrade(&obj.clone()));
            class = maybe_super_class.unwrap();
        } else {
            return Ok(Variable::Reference(original_obj.unwrap()));
        }
    }
}

fn get_class_method_code(class: &ClassResult, target_method_name: &str, target_descriptor: &str) -> Result<Code, RunnerError> {
    let debug = false;
    let class_name = try!(get_cp_class(&class.constant_pool, class.this_class_index));
    let mut method_res: Result<&FieldItem, RunnerError> = Err(RunnerError::ClassInvalid2(format!("Could not find method {} with descriptor {} in class {}", target_method_name, target_descriptor, class_name)));

    for method in &class.methods {
        let method_name = try!(get_cp_str(&class.constant_pool, method.name_index));
        let descriptor = try!(get_cp_str(&class.constant_pool, method.descriptor_index));
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
        return Err(RunnerError::NativeMethod(format!("Method '{}' descriptor '{}' in class '{}'", target_method_name, target_descriptor, class_name)));
    } else {
        let code = try!(method.attributes.iter().filter_map(|x|
            match x {
                &AttributeItem::Code(ref c) => Some(c),
                _ => None
            })
            .nth(0).ok_or(RunnerError::ClassInvalid("Class method has no code")));
        return Ok(code.clone());
    }
}

fn load<F>(desc: &str, index: u8, runtime: &mut Runtime, _t: F) -> Result<(), RunnerError> { // TODO: Type checking
    let loaded = runtime.current_frame.local_variables[index as usize].clone();
    runnerPrint!(runtime, true, 2, "{} {} {}", desc, index, loaded);
    runtime.push_on_stack(loaded);
    return Ok(());
}

fn aload<F, G>(desc: &str, runtime: &mut Runtime, _t: F, converter: G) -> Result<(), RunnerError>
    where G: Fn(Variable) -> Variable
{ // TODO: Type checking
    let index = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap().to_int();
    let var = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    let array_obj = var.to_arrayobj();
    runnerPrint!(runtime, true, 2, "{} {} {}", desc, index, var);
    if array_obj.is_null {
        let exception = try!(construct_object(runtime, &"java/lang/NullPointerException"));
        return Err(RunnerError::Exception(exception));
    }

    let array = array_obj.elements.borrow();
    if array.len() <= index as usize {
        let exception = try!(construct_object(runtime, &"java/lang/ArrayIndexOutOfBoundsException"));
        return Err(RunnerError::Exception(exception));
    }

    let item = converter(array[index as usize].clone());

    runtime.push_on_stack(item);
    return Ok(());
}

fn store<F>(desc: &str, index: u8, runtime: &mut Runtime, _t: F) -> Result<(), RunnerError> { // TODO: Type checking
    let popped = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    runnerPrint!(runtime, true, 2, "{}_{} {}", desc, index, popped);
    while runtime.current_frame.local_variables.len() <= index as usize {
        runtime.current_frame.local_variables.push(Variable::Int(0));
    }
    runtime.current_frame.local_variables[index as usize] = popped;
    return Ok(());
}


fn astore<F>(desc: &str, runtime: &mut Runtime, converter: F) -> Result<(), RunnerError>
    where F: Fn(&Variable) -> Variable
{ // TODO: Type checking
    let value = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    let index = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap().to_int();
    let var = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    let array_obj = var.to_arrayobj();
    runnerPrint!(runtime, true, 2, "{} {} {}", desc, index, var);
    if array_obj.is_null {
        let exception = try!(construct_object(runtime, &"java/lang/NullPointerException"));
        return Err(RunnerError::Exception(exception));
    }

    let mut array = array_obj.elements.borrow_mut();
    if array.len() <= index as usize {
        let exception = try!(construct_object(runtime, &"java/lang/ArrayIndexOutOfBoundsException"));
        return Err(RunnerError::Exception(exception));
    }

    array[index as usize] = converter(&value);
    return Ok(());
}

fn and<F>(a: F, b: F) -> <F as std::ops::BitAnd>::Output where F: BitAnd { a&b }
fn or<F>(a: F, b: F) -> <F as std::ops::BitOr>::Output where F: BitOr { a|b }
fn xor<F>(a: F, b: F) -> <F as std::ops::BitXor>::Output where F: BitXor { a^b }

fn maths_instr<F, G, H, K>(desc: &str, runtime: &mut Runtime, creator: F, extractor: G, operation: H)
    where
    F: Fn(K) -> Variable,
    G: Fn(&Variable) -> K,
    H: Fn(K, K) -> K
{
    let popped1 = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    let popped2 = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    runnerPrint!(runtime, true, 2, "{} {} {}", desc, popped1, popped2);
    runtime.push_on_stack(creator(operation(extractor(&popped2), extractor(&popped1))));
}

fn maths_instr_2<F, G, H, I, J, K, L>(desc: &str, runtime: &mut Runtime, creator: F, extractor1: G, extractor2: H, operation: I)
    where
        F: Fn(L) -> Variable,
        G: Fn(&Variable) -> J,
        H: Fn(&Variable) -> K,
        I: Fn(K, J) -> L
{
    let popped1 = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    let popped2 = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    runnerPrint!(runtime, true, 2, "{} {} {}", desc, popped1, popped2);
    runtime.push_on_stack(creator(operation(extractor2(&popped2), extractor1(&popped1))));
}

fn single_pop_instr<F, G, H, I, J>(desc: &str, runtime: &mut Runtime, creator: F, extractor: G, operation: H)
    where
    F: Fn(J) -> Variable,
    G: Fn(&Variable) -> I,
    H: Fn(I) -> J
{
    let popped = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    runnerPrint!(runtime, true, 2, "{} {}", desc, popped);
    runtime.push_on_stack(creator(operation(extractor(&popped))));
}

fn vreturn<F, K>(desc: &str, runtime: &mut Runtime, extractor: F) -> Result<bool, RunnerError> where F: Fn(&Variable) -> K {
    let popped = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    runnerPrint!(runtime, true, 1, "{} {}", desc, popped);
    extractor(&popped); // Type check
    runtime.current_frame = runtime.previous_frames.pop().unwrap();
    runtime.push_on_stack(popped);
    return Err(RunnerError::Return);
}

// Get the (super)object which contains a field
fn get_obj_field(mut obj: Rc<Object>, field_name: &str) -> Result<Rc<Object>, RunnerError> {
    let class_name = obj.type_ref.name.clone();
    while {let members = obj.members.borrow(); !members.contains_key(field_name) } {
        let new_obj = obj.super_class.borrow().clone();
        if new_obj.is_none() {
            return Err(RunnerError::ClassInvalid2(format!("Couldn't find field '{}' in class {}", field_name, class_name)));
        }
        obj = new_obj.unwrap();
    }
    return Ok(obj.clone());
}

fn get_super_obj(mut obj: Rc<Object>, class_name: &str) -> Result<Rc<Object>, RunnerError> {
    while obj.type_ref.name != class_name && obj.super_class.borrow().is_some() {
        let new_obj = obj.super_class.borrow().clone().unwrap();
        obj = new_obj;
        debugPrint!(false, 3, "Class didn't match, checking '{}' now)", obj.type_ref.name);
    }

    if obj.type_ref.name != class_name {
        debugPrint!(true, 1, "Expected object on stack with class name '{}' but got '{}'", class_name, obj.type_ref.name);
        return Err(RunnerError::ClassInvalid2(format!("Couldn't find object on stack with class name '{}'", class_name)));
    }

    return Ok(obj);
}

pub fn invoke_nested(runtime: &mut Runtime, class: Rc<Class>, args: Vec<Variable>, method_name: &str, method_descriptor: &str, allow_not_found: bool) -> Result<(), RunnerError>{
    let maybe_code = get_class_method_code(&class.cr, method_name, method_descriptor);
    if maybe_code.is_err() {
        if allow_not_found { return Ok(()) }
        else { return Err(maybe_code.err().unwrap()) };
    }
    let new_frame = Frame {
        class: Some(class.clone()),
        constant_pool: class.cr.constant_pool.clone(),
        operand_stack: Vec::new(),
        local_variables: args.clone(),
        name: String::from(class.name.clone() + method_name),
        code: maybe_code.unwrap(),
        return_pos: 0,
    };

    runnerPrint!(runtime, true, 1, "INVOKE manual {} {} on {}", method_name, method_descriptor, class.name);
    runtime.previous_frames.push(runtime.current_frame.clone());
    runtime.current_frame = new_frame;
    return do_run_method(runtime);
}


fn try_builtin(class_name: &Rc<String>, method_name: &Rc<String>, descriptor: &Rc<String>, args: &Vec<Variable>, runtime: &mut Runtime) -> Result<bool, RunnerError> {
    runnerPrint!(runtime, true, 4, "try_builtin {} {} {}", class_name, method_name, descriptor);

    if try!(java_lang::try_builtin(class_name, method_name, descriptor, args, runtime)) {
        return Ok((true));
    }
    if try!(java_other::try_builtin(class_name, method_name, descriptor, args, runtime)) {
        return Ok((true));
    }
    if try!(sun::try_builtin(class_name, method_name, descriptor, args, runtime)) {
        return Ok((true));
    }

    return Ok(false);
}


fn invoke(desc: &str, runtime: &mut Runtime, index: u16, with_obj: bool, special: bool) -> Result<(), RunnerError> {
    let debug = true;
    let mut code : Option<Code>;
    let new_frame : Option<Frame>;
    let new_method_name : Option<String>;
    let current_op_stack_size = runtime.current_frame.operand_stack.len();

    {
        let (class_name, method_name, descriptor) = try!(get_cp_method(&runtime.current_frame.constant_pool, index));
        new_method_name = Some((*class_name).clone() + "/" + method_name.as_str());
        let (parameters, _return_type) = try!(parse_function_type_string(runtime, descriptor.as_str()));
        let extra_parameter = if with_obj {1} else {0};
        let new_local_variables = runtime.current_frame.operand_stack.split_off(current_op_stack_size - parameters.len() - extra_parameter);

        runnerPrint!(runtime, debug, 1, "{} {} {} {}", desc, class_name, method_name, descriptor);

        if try!(try_builtin(&class_name, &method_name, &descriptor, &new_local_variables, runtime)) {
            return Ok(());
        }

        let mut class = try!(load_class(runtime, class_name.as_str()));

        if with_obj {
            let mut obj = new_local_variables[0].to_ref();

            if obj.is_null {
                return Err(RunnerError::ClassInvalid2(format!("Missing obj ref on local var stack for method on {}", class_name)));
            }

            if special {
                while obj.type_ref.name != *class_name {
                    let new_obj = try!(
                        obj.super_class.borrow().as_ref()
                            .ok_or(RunnerError::ClassInvalid2(format!("Couldn't find class {} in tree for {}", class_name, obj.type_ref.name)))
                    ).clone();
                    obj = new_obj;
                }
            } else {
                obj = get_most_sub_class(obj);
            }

            // Find method
            while { code = get_class_method_code(&obj.type_ref.cr, method_name.as_str(), descriptor.as_str()).ok(); code.is_none() } {
                if obj.super_class.borrow().is_none() {
                    return Err(RunnerError::ClassInvalid2(format!("Could not find super class of object '{}' that matched method '{}' '{}'", obj, method_name, descriptor)))
                }
                let new_obj = obj.super_class.borrow().clone().unwrap();
                obj = new_obj;
            }
            class = obj.type_ref.clone();
        } else {
            code = Some(try!(get_class_method_code(&class.cr, method_name.as_str(), descriptor.as_str())));
        }

        new_frame = Some(Frame {
            class: Some(class.clone()),
            constant_pool: class.cr.constant_pool.clone(),
            operand_stack: Vec::new(),
            local_variables: new_local_variables,
            name: new_method_name.unwrap(),
            code: code.unwrap(),
            return_pos: 0,
        });

    }

    runtime.previous_frames.push(runtime.current_frame.clone());
    runtime.current_frame = new_frame.unwrap();
    return Err(RunnerError::Invoke);
}

fn fcmp(desc: &str, runtime: &mut Runtime, is_g: bool) -> Result<(), RunnerError> {
    let pop2 = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap().to_float();
    let pop1 = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap().to_float();
    runnerPrint!(runtime, true, 2, "{} {} {}", desc, pop1, pop2);
    let ret;
    if pop1.is_nan() || pop2.is_nan() {
        ret = if is_g {1} else {-1}
    } else if pop1 > pop2 {
        ret = 1;
    } else if pop1 == pop2 {
        ret = 0;
    } else {
        ret = -1;
    }
    runtime.push_on_stack(Variable::Int(ret));
    return Ok(());
}

fn ifcmp<F>(desc: &str, runtime: &mut Runtime, buf: &mut Cursor<&Vec<u8>>, cmp: F) -> Result<(), RunnerError>
    where F: Fn(i32) -> bool
{
    let current_position = buf.position() - 1;
    let branch_offset = try!(buf.read_u16::<BigEndian>()) as i16;
    let popped = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    runnerPrint!(runtime, true, 2, "{} {} {}", desc, popped, branch_offset);
    if cmp(popped.to_int()) {
        let new_position = (current_position as i64 + branch_offset as i64) as u64;
        runnerPrint!(runtime, true, 2, "BRANCHED from {} to {}", current_position, new_position);
        buf.set_position(new_position);
    }
    return Ok(());
}

fn branch_if<F>(desc: &str, runtime: &mut Runtime, buf: &mut Cursor<&Vec<u8>>, current_position: u64, cmp: F) -> Result<(), RunnerError>
    where F: Fn(&Variable) -> bool
{
    let branch_offset = try!(buf.read_u16::<BigEndian>()) as i16;
    let var = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    let compare_result = cmp(&var);
    runnerPrint!(runtime, true, 2, "{} {} {} {}", desc, var, branch_offset, compare_result);
    if compare_result {
        let new_pos = (current_position as i64 + branch_offset as i64) as u64;
        runnerPrint!(runtime, true, 2, "BRANCHED from {} to {}", current_position, new_pos);
        buf.set_position(new_pos);
    }
    return Ok(());
}

pub fn make_field(runtime: &mut Runtime, clazz: &Variable, name: Rc<String>, descriptor: Rc<String>, _access: u16, slot: i32)  -> Result<Variable, RunnerError> {
    let class_name = "java/lang/reflect/Field";
    let name_var = try!(make_string(runtime, name.as_str()));
    let name_var_interned = try!(string_intern(runtime, &name_var));
    let signature_var = try!(make_string(runtime, descriptor.as_str()));
    let var = try!(construct_object(runtime, class_name));
    try!(put_field(runtime, var.to_ref(), class_name, "name", name_var_interned));
    try!(put_field(runtime, var.to_ref(), class_name, "signature", signature_var));
    let type_obj = try!(make_class(runtime, descriptor.as_str()));
    try!(put_field(runtime, var.to_ref(), class_name, "type", type_obj));
    try!(put_field(runtime, var.to_ref(), class_name, "slot", Variable::Int(slot)));
    try!(put_field(runtime, var.to_ref(), class_name, "clazz", clazz.clone()));
    return Ok(var);
}

pub fn make_method(runtime: &mut Runtime, name: Rc<String>, descriptor: Rc<String>, _access: u16)  -> Result<Variable, RunnerError> {
    let class_name = &"java/lang/reflect/Method";
    let name_var = try!(make_string(runtime, name.as_str()));
    let name_var_interned = try!(string_intern(runtime, &name_var));
    let signature_var = try!(make_string(runtime, descriptor.as_str()));
    let var = try!(construct_object(runtime, class_name));
    try!(put_field(runtime, var.to_ref(), class_name, "name", name_var_interned));
    try!(put_field(runtime, var.to_ref(), class_name, "signature", signature_var));
    return Ok(var);
}

pub fn get_primitive_class(runtime: &mut Runtime, descriptor: String) -> Result<Variable, RunnerError> {
    if descriptor.len() > 1 {
        panic!("Asked to make primitive class of type '{}'", descriptor);
    }

    {
        let maybe_existing = runtime.class_objects.get(&descriptor);
        if maybe_existing.is_some() {
            return Ok(maybe_existing.unwrap().clone());
        }
    }

    let var = try!(construct_object(runtime, &"java/lang/Class"));
    runtime.class_objects.insert(descriptor.clone(), var.clone());

    let name_object = try!(make_string(runtime, try!(descriptor_to_type_name(descriptor.as_str())).as_str()));
    let interned_string = try!(string_intern(runtime, &name_object));
    let statics = &var.to_ref().type_ref.statics;
    statics.borrow_mut().insert(String::from("initted"), Variable::Boolean(true));
    let members = &var.to_ref().members;
    members.borrow_mut().insert(String::from("name"), interned_string);
    members.borrow_mut().insert(String::from("__is_primitive"), Variable::Boolean(true));
    members.borrow_mut().insert(String::from("__is_array"), Variable::Boolean(false));

    return Ok(var);
}

pub fn make_class(runtime: &mut Runtime, descriptor: &str) -> Result<Variable, RunnerError> {
    {
        let maybe_existing = runtime.class_objects.get(&String::from(descriptor));
        if maybe_existing.is_some() {
            return Ok(maybe_existing.unwrap().clone());
        }
    }

    let var = try!(construct_object(runtime, &"java/lang/Class"));
    runtime.class_objects.insert(String::from(descriptor), var.clone());

    let name_object = try!(make_string(runtime, try!(descriptor_to_type_name(descriptor)).as_str()));
    let interned_string = try!(string_intern(runtime, &name_object));
    try!(put_field(runtime, var.to_ref(), &"java/lang/Class", "name", interned_string));
    let statics = &var.to_ref().type_ref.statics;
    statics.borrow_mut().insert(String::from("initted"), Variable::Boolean(true));
    let members = &var.to_ref().members;

    let subtype = try!(parse_single_type_string(runtime, descriptor, false));
    let mut is_primitive = false;
    let mut is_array = false;
    let mut is_unresolved = false;
    match subtype {
        Variable::UnresolvedReference(ref _type_string) => {
            is_unresolved = true;
        },
        Variable::Reference(ref obj) => {
            let class = obj.type_ref.clone();
            members.borrow_mut().insert(String::from("__class"), try!(construct_null_object(runtime, class)));
        },
        Variable::ArrayReference(ref array_obj) => {
            is_array = true;
            let component_type;
            if array_obj.element_type_ref.is_some() {
                component_type = try!(make_class(runtime, array_obj.element_type_str.clone().as_str()));
            } else {
                component_type = try!(get_primitive_class(runtime, array_obj.element_type_str.clone()));
            }
            members.borrow_mut().insert(String::from("__componentType"), component_type);
        },
        _ => { is_primitive = true; }
    }
    members.borrow_mut().insert(String::from("__is_primitive"), Variable::Boolean(is_primitive));
    members.borrow_mut().insert(String::from("__is_array"), Variable::Boolean(is_array));
    members.borrow_mut().insert(String::from("__is_unresolved"), Variable::Boolean(is_unresolved));

    return Ok(var);
}

fn put_static(runtime: &mut Runtime, class_name: &str, field_name: &str, value: Variable) -> Result<(), RunnerError> {
    let debug = false;
    runnerPrint!(runtime, debug, 2, "Put Static Field {} {} {}", class_name, field_name, value);
    let class_result = try!(load_class(runtime, class_name));
    let mut statics = class_result.statics.borrow_mut();
    if !statics.contains_key(field_name) {
        return Err(RunnerError::ClassInvalid2(format!("Couldn't find static '{}' in class '{}' to put", field_name, class_name)));;
    }
    statics.insert(String::from(field_name), value);
    return Ok(());
}

pub fn put_field(runtime: &mut Runtime, obj: Rc<Object>, class_name: &str, field_name: &str, value: Variable) -> Result<(), RunnerError> {
    let debug = false;
    runnerPrint!(runtime, debug, 2, "Put Field {} {} {}", class_name, field_name, value);
    let super_obj = try!(get_super_obj(obj, class_name));
    let super_obj_with_field = try!(get_obj_field(super_obj, field_name));
    let mut members = super_obj_with_field.members.borrow_mut();
    members.insert(String::from(field_name), value);
    return Ok(());
}

pub fn get_field(runtime: &mut Runtime, obj: &Rc<Object>, class_name: &str, field_name: &str) -> Result<Variable, RunnerError> {
    let debug = false;

    runnerPrint!(runtime, debug, 2, "Get Field {} {} {}", *obj, class_name, field_name);

    if obj.is_null {
        let exception = try!(construct_object(runtime, &"java/lang/NullPointerException"));
        return Err(RunnerError::Exception(exception));
    }

    let super_obj = try!(get_super_obj(obj.clone(), class_name));
    let super_obj_with_field = try!(get_obj_field(super_obj, field_name));
    let mut members = super_obj_with_field.members.borrow_mut();

    let unresolved_type_name;
    {
        let member = members.get(&*field_name).unwrap();
        if !member.is_unresolved() {
            return Ok(member.clone());
        }
        unresolved_type_name = member.get_unresolved_type_name().clone();
    }

    let var = try!(construct_null_object_by_name(runtime, unresolved_type_name.as_str()));
    members.insert(String::from(field_name), var.clone());
    return Ok(var);
}

fn icmp<F>(desc: &str, runtime: &mut Runtime, buf: &mut Cursor<&Vec<u8>>, cmp: F) -> Result<(), RunnerError>
    where F: Fn(i32, i32) -> bool
{
    let current_position = buf.position() - 1;
    let branch_offset = try!(buf.read_u16::<BigEndian>()) as i16;
    let popped2 = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    let popped1 = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    runnerPrint!(runtime, true, 2, "{} {} {} {}", desc, popped1, popped2, branch_offset);
    if cmp(popped1.to_int(), popped2.to_int()) {
        let new_position = (current_position as i64 + branch_offset as i64) as u64;
        runnerPrint!(runtime, true, 2, "BRANCHED from {} to {}", current_position, new_position);
        buf.set_position(new_position);
    }
    return Ok(());
}

fn cast<F>(desc: &str, runtime: &mut Runtime, mutator: F)
    where F: Fn(&Variable) -> Variable
{
    let popped = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    runnerPrint!(runtime, true, 2, "{} {}", desc, popped);
    runtime.push_on_stack(mutator(&popped));
}

fn ifacmp(desc: &str, runtime: &mut Runtime, buf: &mut Cursor<&Vec<u8>>, should_match: bool) -> Result<(), RunnerError>
{
    let current_position = buf.position() - 1;
    let branch_offset = try!(buf.read_u16::<BigEndian>()) as i16;
    let popped2 = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    let popped1 = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    runnerPrint!(runtime, true, 2, "{} {} {} {}", desc, popped1, popped2, branch_offset);
    let matching = match popped1 {
        Variable::Reference(ref obj1) => {
            match popped2 {
                Variable::Reference(ref obj2) => {
                    (obj1.is_null && obj2.is_null) || rc_ptr_eq(obj1, obj2)
                },
                _ => false
            }
        },
        Variable::ArrayReference(ref aobj1) => {
            match popped2 {
                Variable::ArrayReference(ref aobj2) => {
                    (aobj1.is_null && aobj2.is_null) || rc_ptr_eq(aobj1, aobj2)
                },
                _ => false
            }
        },
        _ => false
    };
    if should_match == matching {
        let new_position = (current_position as i64 + branch_offset as i64) as u64;
        runnerPrint!(runtime, true, 2, "BRANCHED from {} to {}", current_position, new_position);
        buf.set_position(new_position);
    }
    return Ok(());
}

fn ldc(runtime: &mut Runtime, index: usize) -> Result<(), RunnerError> {
    let maybe_cp_entry = runtime.current_frame.constant_pool.get(&(index as u16)).map(|x| x.clone());
    if maybe_cp_entry.is_none() {
        runnerPrint!(runtime, true, 1, "LDC failed at index {}", index);
        return Err(RunnerError::ClassInvalid2(format!("LDC failed at index {}", index)));
    } else {
        match maybe_cp_entry.as_ref().unwrap() {
            &ConstantPoolItem::CONSTANT_String { index } => {
                let str = try!(get_cp_str(&runtime.current_frame.constant_pool, index));
                runnerPrint!(runtime, true, 2, "LDC string {}", str);
                let var = try!(make_string(runtime, str.as_str()));
                runtime.push_on_stack(var);
            }
            &ConstantPoolItem::CONSTANT_Class { index } => {
                let constant_pool_descriptor = try!(get_cp_str(&runtime.current_frame.constant_pool, index));
                // Class descriptors are either:
                // "ClassName"
                // or
                // "[[[[I"
                // or
                // "[[[[LClassName;"
                // We first normalise this to a standard descriptor. Note we know it cannot be primitive
                let mut descriptor;
                if constant_pool_descriptor.chars().nth(0).unwrap() == '[' {
                    descriptor = (*constant_pool_descriptor).clone();
                } else {
                    descriptor = 'L'.to_string();
                    descriptor.push_str(constant_pool_descriptor.as_str());
                    descriptor.push(';');
                }
                runnerPrint!(runtime, true, 2, "LDC class {}", descriptor);
                let var = try!(make_class(runtime, descriptor.as_str()));
                runtime.push_on_stack(var);
            }
            &ConstantPoolItem::CONSTANT_Integer { value } => {
                runnerPrint!(runtime, true, 2, "LDC int {}", value as i32);
                runtime.push_on_stack(Variable::Int(value as i32));
            }
            &ConstantPoolItem::CONSTANT_Float { value } => {
                runnerPrint!(runtime, true, 2, "LDC float {}", value as f32);
                runtime.push_on_stack(Variable::Float(value as f32));
            }
            _ => return Err(RunnerError::ClassInvalid2(format!("Unknown constant {:?}", maybe_cp_entry.as_ref().unwrap())))
        }
    }
    return Ok(());
}

fn instruction(runtime: &mut Runtime, name: &str, buf: &mut Cursor<&Vec<u8>>) -> Result<bool, RunnerError> {
    let current_position = buf.position();
    let op_code = try!(buf.read_u8());
    runnerPrint!(runtime, true, 3, "{} {} Op code {}", name, runtime.count, op_code);
    runtime.count+=1;
    match op_code {
        1 => {
            runnerPrint!(runtime, true, 2, "ACONST_NULL");
            let obj = try!(construct_null_object_by_name(runtime, "java/lang/Object"));
            runtime.push_on_stack(obj);
        }
        2...8 => {
            let val = (op_code as i32) - 3;
            runnerPrint!(runtime, true, 2, "ICONST {}", val);
            runtime.push_on_stack(Variable::Int(val));
        }
        9...10 => {
            let val = (op_code as i64) - 9;
            runnerPrint!(runtime, true, 2, "LCONST {}", val);
            runtime.push_on_stack(Variable::Long(val));
        }
        11...13 => {
            let val = (op_code - 11) as f32;
            runnerPrint!(runtime, true, 2, "FCONST {}", val);
            runtime.push_on_stack(Variable::Float(val));
        }
        16 => {
            let byte = try!(buf.read_u8()) as i32;
            runnerPrint!(runtime, true, 2, "BIPUSH {}", byte);
            runtime.push_on_stack(Variable::Int(byte));
        }
        17 => {
            let short = try!(buf.read_u16::<BigEndian>()) as i32;
            runnerPrint!(runtime, true, 2, "SIPUSH {}", short);
            runtime.push_on_stack(Variable::Int(short));
        }
        18 => { // LDC
            let index = try!(buf.read_u8());
            try!(ldc(runtime, index as usize));
        },
        19 => {
            let index = try!(buf.read_u16::<BigEndian>());
            try!(ldc(runtime, index as usize));
        }
        20 => { // LDC2W
            let index = try!(buf.read_u16::<BigEndian>());
            let maybe_cp_entry = runtime.current_frame.constant_pool.get(&(index as u16)).map(|x| x.clone());
            if maybe_cp_entry.is_none() {
                runnerPrint!(runtime, true, 1, "LDC2W failed at index {}", index);
                return Err(RunnerError::ClassInvalid2(format!("LDC2W failed at index {}", index)));
            } else {
                match maybe_cp_entry.as_ref().unwrap() {
                    &ConstantPoolItem::CONSTANT_Long { value } => {
                        runnerPrint!(runtime, true, 2, "LDC2W long {}", value);
                        runtime.push_on_stack(Variable::Long(value as i64));
                    }
                    &ConstantPoolItem::CONSTANT_Double { value } => {
                        runnerPrint!(runtime, true, 2, "LDC2W double {}", value);
                        runtime.push_on_stack(Variable::Double(value));
                    }
                    _ => return Err(RunnerError::ClassInvalid2(format!("Invalid constant for LDC2W {:?}", maybe_cp_entry.as_ref().unwrap())))
                }
            }
        },
        21 => try!(load("ILOAD", try!(buf.read_u8()), runtime, Variable::Int)),
        22 => try!(load("LLOAD", try!(buf.read_u8()), runtime, Variable::Long)),
        23 => try!(load("FLOAD", try!(buf.read_u8()), runtime, Variable::Float)),
        24 => try!(load("DLOAD", try!(buf.read_u8()), runtime, Variable::Double)),
        25 => try!(load("ALOAD", try!(buf.read_u8()), runtime, Variable::Reference)),
        26...29 => try!(load("ILOAD", op_code - 26, runtime, Variable::Int)),
        30...33 => try!(load("LLOAD", op_code - 30, runtime, Variable::Long)),
        34...37 => try!(load("FLOAD", op_code - 34, runtime, Variable::Float)),
        38...41 => try!(load("DLOAD", op_code - 38, runtime, Variable::Double)),
        42...45 => try!(load("ALOAD", op_code - 42, runtime, Variable::Reference)),
        46 => try!(aload("IALOAD", runtime, Variable::Int, |x| x)),
        47 => try!(aload("LALOAD", runtime, Variable::Long, |x| x)),
        48 => try!(aload("FALOAD", runtime, Variable::Float, |x| x)),
        49 => try!(aload("DALOAD", runtime, Variable::Double, |x| x)),
        50 => try!(aload("AALOAD", runtime, Variable::Reference, |x| x)),
        51 => try!(aload("BALOAD", runtime, Variable::Byte, |x| x)),
        52 => try!(aload("CALOAD", runtime, Variable::Char, |x| Variable::Int(Variable::to_int(&x)))),
        53 => try!(aload("SALOAD", runtime, Variable::Short, |x| x)),
        54 => try!(store("ISTORE", try!(buf.read_u8()), runtime, Variable::Int)),
        55 => try!(store("LSTORE", try!(buf.read_u8()), runtime, Variable::Long)),
        56 => try!(store("FSTORE", try!(buf.read_u8()), runtime, Variable::Float)),
        57 => try!(store("DSTORE", try!(buf.read_u8()), runtime, Variable::Double)),
        58 => try!(store("ASTORE", try!(buf.read_u8()), runtime, Variable::Reference)),
        59...62 => try!(store("ISTORE", op_code - 59, runtime, Variable::Int)),
        63...66 => try!(store("LSTORE", op_code - 63, runtime, Variable::Long)),
        67...70 => try!(store("FSTORE", op_code - 67, runtime, Variable::Float)),
        71...74 => try!(store("DSTORE", op_code - 71, runtime, Variable::Double)),
        75...78 => try!(store("ASTORE", op_code - 75, runtime, Variable::Reference)),
        79 => try!(astore("IASTORE", runtime, |x| x.clone())),
        80 => try!(astore("LASTORE", runtime, |x| x.clone())),
        81 => try!(astore("FASTORE", runtime, |x| x.clone())),
        82 => try!(astore("DASTORE", runtime, |x| x.clone())),
        83 => try!(astore("AASTORE", runtime, |x| x.clone())),
        84 => try!(astore("BASTORE", runtime, |x| Variable::Byte(x.to_int() as u8))),
        85 => try!(astore("CASTORE", runtime, |x| Variable::Char(std::char::from_u32((x.to_int() as u32) & 0xFF).unwrap()))),
        86 => try!(astore("SASTORE", runtime, |x| Variable::Short(x.to_int() as i16))),
        87 => {
            let popped = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
            runnerPrint!(runtime, true, 2, "POP {}", popped);
        }
        88 => {
            let popped = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
            if popped.is_type_1() {
                let popped2 = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
                runnerPrint!(runtime, true, 2, "POP2 {} {}", popped, popped2);
            } else {
                runnerPrint!(runtime, true, 2, "POP2 {}", popped);
            }
        }
        89 => {
            let stack_len = runtime.current_frame.operand_stack.len();
            let peek = runtime.current_frame.operand_stack[stack_len - 1].clone();
            runnerPrint!(runtime, true, 2, "DUP {}", peek);
            runtime.push_on_stack(peek);
        }
        90 => {
            let stack_len = runtime.current_frame.operand_stack.len();
            let peek = runtime.current_frame.operand_stack[stack_len - 1].clone();
            runnerPrint!(runtime, true, 2, "DUP_X1 {}", peek);
            runtime.current_frame.operand_stack.insert(stack_len - 2, peek);
        }
        91 => {
            let stack_len = runtime.current_frame.operand_stack.len();
            let peek = runtime.current_frame.operand_stack[stack_len - 1].clone();
            runnerPrint!(runtime, true, 2, "DUP_X2 {}", peek);
            runtime.current_frame.operand_stack.insert(stack_len - 3, peek);
        }
        92 => {
            let stack_len = runtime.current_frame.operand_stack.len();
            let peek1 = runtime.current_frame.operand_stack[stack_len - 1].clone();
            if peek1.is_type_1() {
                let peek2 = runtime.current_frame.operand_stack[stack_len - 2].clone();
                runnerPrint!(runtime, true, 2, "DUP2 {} {}", peek1, peek2);
                runtime.push_on_stack(peek2);
                runtime.push_on_stack(peek1);
            } else {
                runnerPrint!(runtime, true, 2, "DUP2 {}", peek1);
                runtime.push_on_stack(peek1);
            }
        }
        96 => maths_instr("IADD", runtime, Variable::Int, Variable::to_int, i32::wrapping_add),
        97 => maths_instr("LADD", runtime, Variable::Long, Variable::to_long, i64::wrapping_add),
        98 => maths_instr("FADD", runtime, Variable::Float, Variable::to_float, std::ops::Add::add),
        99 => maths_instr("DADD", runtime, Variable::Double, Variable::to_double, std::ops::Add::add),
        100 => maths_instr("ISUB", runtime, Variable::Int, Variable::to_int, i32::wrapping_sub),
        101 => maths_instr("LSUB", runtime, Variable::Long, Variable::to_long, i64::wrapping_sub),
        102 => maths_instr("FSUB", runtime, Variable::Float, Variable::to_float, std::ops::Sub::sub),
        103 => maths_instr("DSUB", runtime, Variable::Double, Variable::to_double, std::ops::Sub::sub),
        104 => maths_instr("IMUL", runtime, Variable::Int, Variable::to_int, i32::wrapping_mul),
        105 => maths_instr("LMUL", runtime, Variable::Long, Variable::to_long, i64::wrapping_mul),
        106 => maths_instr("FMUL", runtime, Variable::Float, Variable::to_float, std::ops::Mul::mul),
        107 => maths_instr("DMUL", runtime, Variable::Double, Variable::to_double, std::ops::Mul::mul),
        108 => maths_instr("IDIV", runtime, Variable::Int, Variable::to_int, i32::wrapping_div),
        109 => maths_instr("LDIV", runtime, Variable::Long, Variable::to_long, i64::wrapping_div),
        110 => maths_instr("FDIV", runtime, Variable::Float, Variable::to_float, std::ops::Div::div),
        111 => maths_instr("DDIV", runtime, Variable::Double, Variable::to_double, std::ops::Div::div),
        112 => maths_instr("IREM", runtime, Variable::Int, Variable::to_int, i32::wrapping_rem),
        113 => maths_instr("LREM", runtime, Variable::Long, Variable::to_long, i64::wrapping_rem),
        114 => maths_instr("FREM", runtime, Variable::Float, Variable::to_float, std::ops::Rem::rem),
        115 => maths_instr("DREM", runtime, Variable::Double, Variable::to_double, std::ops::Rem::rem),
        116 => single_pop_instr("INEG", runtime, Variable::Int, Variable::to_int, |x| 0 - x),
        117 => single_pop_instr("LNEG", runtime, Variable::Long, Variable::to_long, |x| 0 - x),
        118 => single_pop_instr("FNEG", runtime, Variable::Float, Variable::to_float, |x| 0.0 - x),
        119 => single_pop_instr("DNEG", runtime, Variable::Double, Variable::to_double, |x| 0.0 - x),
        120 => maths_instr("ISHL", runtime, Variable::Int, Variable::to_int, |x,y| x << y),
        121 => maths_instr_2("LSHL", runtime, Variable::Long, Variable::to_int, Variable::to_long, |x,y| (x << y) as i64),
        122 => maths_instr("ISHR", runtime, Variable::Int, Variable::to_int, |x,y| x >> y),
        123 => maths_instr_2("LSHR", runtime, Variable::Long, Variable::to_int, Variable::to_long, |x,y| (x >> y) as i64),
        124 => maths_instr("IUSHR", runtime, Variable::Int, Variable::to_int, |x,y| ((x as u32)>>y) as i32),
        125 => maths_instr_2("LUSHR", runtime, Variable::Long, Variable::to_int, Variable::to_long, |x,y| ((x as u64)>>y) as i64),
        126 => maths_instr("IAND", runtime, Variable::Int, Variable::to_int, and),
        127 => maths_instr("LAND", runtime, Variable::Long, Variable::to_long, and),
        128 => maths_instr("IOR", runtime, Variable::Int, Variable::to_int, or),
        129 => maths_instr("LOR", runtime, Variable::Long, Variable::to_long, or),
        130 => maths_instr("IXOR", runtime, Variable::Int, Variable::to_int, xor),
        131 => maths_instr("LXOR", runtime, Variable::Long, Variable::to_long, xor),
        132 => {
            let index = try!(buf.read_u8());
            let constt = try!(buf.read_u8()) as i8;
            runnerPrint!(runtime, true, 2, "IINC {} {}", index, constt);
            let old_val = runtime.current_frame.local_variables[index as usize].to_int();
            runtime.current_frame.local_variables[index as usize] = Variable::Int(old_val + constt as i32);
        }
        133 => cast("I2L", runtime, |x| Variable::Long(x.to_int() as i64)),
        134 => cast("I2F", runtime, |x| Variable::Float(x.to_int() as f32)),
        135 => cast("I2D", runtime, |x| Variable::Double(x.to_int() as f64)),
        136 => single_pop_instr("L2I", runtime, Variable::Int, Variable::to_long, |x| x as i32),
        139 => cast("F2I", runtime, |x| Variable::Int(x.to_float() as i32)),
        140 => cast("F2L", runtime, |x| Variable::Long(x.to_float() as i64)),
        141 => cast("F2D", runtime, |x| Variable::Double(x.to_float() as f64)),
        142 => cast("D2I", runtime, |x| Variable::Int(x.to_double() as i32)),
        143 => cast("D2L", runtime, |x| Variable::Long(x.to_double() as i64)),
        144 => cast("D2F", runtime, |x| Variable::Float(x.to_double() as f32)),
        145 => cast("I2B", runtime, |x| Variable::Byte(x.to_int() as u8)),
        146 => cast("I2C", runtime, |x| Variable::Char(std::char::from_u32(x.to_int() as u32).unwrap_or('\0'))),
        147 => cast("I2S", runtime, |x| Variable::Short(x.to_int() as i16)),
        148 => {
            let pop2 = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap().to_long();
            let pop1 = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap().to_long();
            runnerPrint!(runtime, true, 2, "LCMP {} {}", pop1, pop2);
            let ret;
            if pop1 > pop2 {
                ret = 1;
            } else if pop1 == pop2 {
                ret = 0;
            } else {
                ret = -1;
            }
            runtime.push_on_stack(Variable::Int(ret));
        }
        149 => try!(fcmp("FCMPG", runtime, true)),
        150 => try!(fcmp("FCMPL", runtime, false)),
        153 => try!(ifcmp("IFEQ", runtime, buf, |x| x == 0)),
        154 => try!(ifcmp("IFNE", runtime, buf, |x| x != 0)),
        155 => try!(ifcmp("IFLT", runtime, buf, |x| x < 0)),
        156 => try!(ifcmp("IFGE", runtime, buf, |x| x >= 0)),
        157 => try!(ifcmp("IFGT", runtime, buf, |x| x > 0)),
        158 => try!(ifcmp("IFLE", runtime, buf, |x| x <= 0)),
        159 => try!(icmp("IF_ICMPEQ", runtime, buf, |x,y| x == y)),
        160 => try!(icmp("IF_ICMPNE", runtime, buf, |x,y| x != y)),
        161 => try!(icmp("IF_ICMPLT", runtime, buf, |x,y| x < y)),
        162 => try!(icmp("IF_ICMPGE", runtime, buf, |x,y| x >= y)),
        163 => try!(icmp("IF_ICMPGT", runtime, buf, |x,y| x > y)),
        164 => try!(icmp("IF_ICMPLE", runtime, buf, |x,y| x <= y)),
        165 => try!(ifacmp("IF_ACMPEQ", runtime, buf, true)),
        166 => try!(ifacmp("IF_ACMPNEQ", runtime, buf, false)),
        167 => {
            let branch_offset = try!(buf.read_u16::<BigEndian>()) as i16;
            let new_pos = (current_position as i64 + branch_offset as i64) as u64;
            runnerPrint!(runtime, true, 2, "BRANCH from {} to {}", current_position, new_pos);
            buf.set_position(new_pos);
        }
        170 => {
            let pos = buf.position();
            buf.set_position((pos + 3) & !3);
            let default = try!(buf.read_u32::<BigEndian>());
            let low = try!(buf.read_u32::<BigEndian>());
            let high = try!(buf.read_u32::<BigEndian>());
            let value_int = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap().to_int() as u32;
            runnerPrint!(runtime, true, 2, "TABLESWITCH {} {} {} {}", default, low, high, value_int);
            if value_int < low || value_int > high {
                let new_pos = (current_position as i64 + default as i64) as u64;
                runnerPrint!(runtime, true, 2, "No match so BRANCH from {} to {}", current_position, new_pos);
                buf.set_position(new_pos);
            } else {
                let pos = buf.position();
                buf.set_position(pos + (value_int - low) as u64 * 4);
                let jump = try!(buf.read_u32::<BigEndian>());
                let new_pos = (current_position as i64 + jump as i64) as u64;
                runnerPrint!(runtime, true, 2, "Match so BRANCH from {} to {}", current_position, new_pos);
                buf.set_position(new_pos);
            }
        }
        171 => {
            let pos = buf.position();
            buf.set_position((pos + 3) & !3);
            let default = try!(buf.read_u32::<BigEndian>());
            let npairs = try!(buf.read_u32::<BigEndian>());
            let value_int = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap().to_int();
            runnerPrint!(runtime, true, 2, "LOOKUPSWITCH {} {} {}", default, npairs, value_int);
            let mut matched = false;
            for _i in 0..npairs { // TODO: Nonlinear search
                let match_key = try!(buf.read_u32::<BigEndian>()) as i32;
                let offset = try!(buf.read_u32::<BigEndian>()) as i32;
                if match_key == value_int {
                    let new_pos = (current_position as i64 + offset as i64) as u64;
                    runnerPrint!(runtime, true, 2, "Matched so BRANCH from {} to {}", current_position, new_pos);
                    buf.set_position(new_pos);
                    matched = true;
                    break;
                }
            }
            if matched == false {
                let new_pos = (current_position as i64 + default as i64) as u64;
                runnerPrint!(runtime, true, 2, "No match so BRANCH from {} to {}", current_position, new_pos);
                buf.set_position(new_pos);
            }
        }
        172 => { return vreturn("IRETURN", runtime, Variable::can_convert_to_int); }
        173 => { return vreturn("LRETURN", runtime, Variable::to_long); }
        174 => { return vreturn("FRETURN", runtime, Variable::to_float); }
        175 => { return vreturn("DRETURN", runtime, Variable::to_double); }
        176 => { return vreturn("ARETURN", runtime, Variable::is_ref_or_array); }
        177 => { // return
            runnerPrint!(runtime, true, 1, "RETURN");
            runtime.current_frame = runtime.previous_frames.pop().unwrap();
            return Err(RunnerError::Return);
        }
        178 => { // getstatic
            let index = try!(buf.read_u16::<BigEndian>());
            let (class_name, field_name, typ) = try!(get_cp_field(&runtime.current_frame.constant_pool, index));
            runnerPrint!(runtime, true, 2, "GETSTATIC {} {} {}", class_name, field_name, typ);
            let mut class_result = try!(load_class(runtime, class_name.as_str()));
            loop {
                {
                    let statics = class_result.statics.borrow();
                    let maybe_static_variable = statics.get(&*field_name);
                    if maybe_static_variable.is_some() {
                        runnerPrint!(runtime, true, 2, "GETSTATIC found {}", maybe_static_variable.unwrap());
                        runtime.push_on_stack(maybe_static_variable.unwrap().clone());
                        break;
                    }
                }
                let maybe_super = class_result.super_class.borrow().clone();
                if maybe_super.is_none() {
                    return Err(RunnerError::ClassInvalid2(format!("Couldn't find static {} in {}", field_name.as_str(), class_name.as_str())));
                }
                class_result = maybe_super.unwrap();
            }
        }
        179 => { // putstatic
            let index = try!(buf.read_u16::<BigEndian>());
            let value = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
            let (class_name, field_name, typ) = try!(get_cp_field(&runtime.current_frame.constant_pool, index));
            runnerPrint!(runtime, true, 2, "PUTSTATIC {} {} {} {}", class_name, field_name, typ, value);
            try!(put_static(runtime, class_name.as_str(), field_name.as_str(), value));
        }
        180 => {
            let field_index = try!(buf.read_u16::<BigEndian>());
            let (class_name, field_name, typ) = try!(get_cp_field(&runtime.current_frame.constant_pool, field_index));
            let var = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
            let obj = var.to_ref();
            let f = try!(get_field(runtime, &obj, class_name.as_str(), field_name.as_str()));
            runnerPrint!(runtime, true, 2, "GETFIELD class:'{}' field:'{}' type:'{}' object:'{}' result:'{}'", class_name, field_name, typ, obj, f);
            runtime.push_on_stack(f);
        }
        181 => {
            let field_index = try!(buf.read_u16::<BigEndian>());
            let (class_name, field_name, typ) = try!(get_cp_field(&runtime.current_frame.constant_pool, field_index));
            let value = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
            let var = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
            let obj = var.to_ref();
            runnerPrint!(runtime, true, 2, "PUTFIELD {} {} {} {} {}", class_name, field_name, typ, obj, value);
            try!(put_field(runtime, obj, class_name.as_str(), field_name.as_str(), value));
        }
        182 => {
            let index = try!(buf.read_u16::<BigEndian>());
            try!(invoke("INVOKEVIRTUAL", runtime, index, true, false));
        },
        183 => {
            let index = try!(buf.read_u16::<BigEndian>());
            try!(invoke("INVOKESPECIAL", runtime, index, true, true));
        },
        184 => {
            let index = try!(buf.read_u16::<BigEndian>());
            try!(invoke("INVOKESTATIC", runtime, index, false, true));
        }
        185 => {
            let index = try!(buf.read_u16::<BigEndian>());
            let _count = try!(buf.read_u8());
            let _zero = try!(buf.read_u8());
            try!(invoke("INVOKEINTERFACE", runtime, index, true, false));
        }
        187 => {
            let index = try!(buf.read_u16::<BigEndian>());
            let class_name = try!(get_cp_class(&runtime.current_frame.constant_pool, index));
            runnerPrint!(runtime, true, 2, "NEW {}", class_name);
            let var = try!(construct_object(runtime, class_name.as_str()));
            runtime.push_on_stack(var);
        }
        188 => {
            let atype = try!(buf.read_u8());
            let count = try!(pop_from_stack(&mut runtime.current_frame.operand_stack).ok_or(RunnerError::ClassInvalid("NEWARRAY POP fail"))).to_int();
            runnerPrint!(runtime, true, 2, "NEWARRAY {} {}", atype, count);

            let var : Variable;
            let type_str : char;
            match atype {
                4 => { var = Variable::Boolean(false); type_str = 'Z'; },
                5 => { var = Variable::Char('\0'); type_str = 'C'; },
                6 => { var = Variable::Float(0.0); type_str = 'F'; },
                7 => { var = Variable::Double(0.0); type_str = 'D'; },
                8 => { var = Variable::Byte(0); type_str = 'B'; },
                9 => { var = Variable::Short(0); type_str = 'S'; },
                10 => { var = Variable::Int(0); type_str = 'I'; },
                11 => { var = Variable::Long(0); type_str = 'J'; },
                _ => return Err(RunnerError::ClassInvalid2(format!("New array type {} unknown", atype)))
            }

            let mut v : Vec<Variable> = Vec::new();
            for _c in 0..count {
                v.push(var.clone());
            }
            let array_obj = try!(construct_primitive_array(runtime, type_str.to_string().as_str(), Some(v)));
            runtime.push_on_stack(array_obj);
        }
        189 => {
            let index = try!(buf.read_u16::<BigEndian>());
            let class_name = try!(get_cp_class(&runtime.current_frame.constant_pool, index));
            try!(load_class(runtime, class_name.as_str()));
            let class = runtime.classes.get(&*class_name).unwrap().clone();
            let count = try!(pop_from_stack(&mut runtime.current_frame.operand_stack).ok_or(RunnerError::ClassInvalid("ANEWARRAY count fail"))).to_int();
            runnerPrint!(runtime, true, 2, "ANEWARRAY {} {}", class_name, count);
            let mut v : Vec<Variable> = Vec::new();
            for _c in 0..count {
                v.push(try!(construct_null_object(runtime, class.clone())));
            }
            let array_obj = try!(construct_array(runtime, class, Some(v)));
            runtime.push_on_stack(array_obj);
        }
        190 => {
            let var = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
            let array_obj = var.to_arrayobj();
            if array_obj.is_null {
                let exception = try!(construct_object(runtime, &"java/lang/NullPointerException"));
                return Err(RunnerError::Exception(exception));
            }
            let len = array_obj.elements.borrow().len();
            runnerPrint!(runtime, true, 2, "ARRAYLEN {} {} {}", var, array_obj.element_type_str, len);
            runtime.push_on_stack(Variable::Int(len as i32));
        }
        192 => {
            let var = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
            let index = try!(buf.read_u16::<BigEndian>());

            runnerPrint!(runtime, true, 2, "CHECKCAST {} {}", var, index);

            {
                let maybe_cp_entry = runtime.current_frame.constant_pool.get(&index);
                if maybe_cp_entry.is_none() {
                    runnerPrint!(runtime, true, 1, "Missing CP class {}", index);
                    return Err(RunnerError::ClassInvalid2(format!("Missing CP class {}", index)));
                }
            }

            // TODO: CHECKCAST (noop)
            runtime.push_on_stack(var);
        }
        193 => {
            let var = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
            let index = try!(buf.read_u16::<BigEndian>());
            let class_name = try!(get_cp_class(&runtime.current_frame.constant_pool, index));

            runnerPrint!(runtime, true, 2, "INSTANCEOF {} {}", var, class_name);

            let var_ref = var.to_ref();
            let mut matches = false;
            if !var_ref.is_null {
                let mut obj = get_most_sub_class(var_ref);

                // Search down to find if instance of
                while {matches = obj.type_ref.name == *class_name; obj.super_class.borrow().is_some()} {
                    if matches {
                        break;
                    }
                    let new_obj = obj.super_class.borrow().as_ref().unwrap().clone();
                    obj = new_obj;
                }
            }
            runtime.push_on_stack(Variable::Int(if matches {1} else {0}));
        }
        194 => {
            let var = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
            runnerPrint!(runtime, true, 2, "MONITORENTER {}", var);
            let _obj = var.to_ref();
            // TODO: Implement monitor
            runnerPrint!(runtime, true, 1, "WARNING: MonitorEnter not implemented");
        },
        195 => {
            let var = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
            runnerPrint!(runtime, true, 2, "MONITOREXIT {}", var);
            let _obj = var.to_ref();
            // TODO: Implement monitor
            runnerPrint!(runtime, true, 1, "WARNING: MonitorExit not implemented");
        },
        198 => try!(branch_if("IFNULL", runtime, buf, current_position, |x| x.is_null())),
        199 => try!(branch_if("IFNONNULL", runtime, buf, current_position, |x| !x.is_null())),
        _ => return Err(RunnerError::UnknownOpCode(op_code))
    }
    return Ok(false);
}

fn do_run_method(runtime: &mut Runtime) -> Result<(), RunnerError> {
    let start_frames = runtime.previous_frames.len();

    loop {
        let code = runtime.current_frame.code.clone();
        let mut buf = Cursor::new(&code.code);
        let name_string = runtime.current_frame.name.clone();

        buf.set_position(runtime.current_frame.return_pos);

        loop {
            let current_position = buf.position();
            let result = instruction(runtime, name_string.as_str(), &mut buf);
            if result.is_err() {
                let mut caught = false;
                let err = result.err().unwrap();
                match &err {
                    &RunnerError::Exception(ref exception) => {
                        runnerPrint!(runtime, true, 3, "Exception {}", exception);
                        for e in &runtime.current_frame.code.exceptions.clone() {
                            if current_position >= e.start_pc as u64 && current_position <= e.end_pc as u64 {
                                if e.catch_type > 0 {
                                    let class_name = try!(get_cp_class(&runtime.current_frame.constant_pool, e.catch_type));
                                    if exception.to_ref().type_ref.name != *class_name {
                                        continue;
                                    }
                                }

                                runnerPrint!(runtime, true, 3, "Caught exception and branching to {}", e.handler_pc);

                                caught = true;
                                runtime.push_on_stack(exception.clone());
                                buf.set_position(e.handler_pc as u64);
                                break;
                            }
                        }
                    },
                    &RunnerError::Invoke => {
                        let len = runtime.previous_frames.len();
                        runtime.previous_frames[len - 1].return_pos = buf.position();
                        break;
                    },
                    &RunnerError::Return => {
                        if runtime.previous_frames.len() < start_frames {
                            return Ok(());
                        }
                        break;
                    }
                    _ => {}
                }

                if caught == false {
                    return Err(err);
                }
            }
        }
    }
}

fn find_class(runtime: &mut Runtime, base_name: &str) -> Result<ClassResult, RunnerError> {
    let debug = false;
    let mut name = String::from(base_name);
    name = name.replace('.', "/");
    runnerPrint!(runtime, debug, 3, "Finding class {}", name);
    for class_path in runtime.class_paths.iter() {
        let mut direct_path = PathBuf::from(class_path);
        for sub in name.split('/') {
            direct_path.push(sub)
        }
        direct_path.set_extension("class");
        runnerPrint!(runtime, debug, 3, "Trying path {}", direct_path.display());
        let direct_classname = get_classname(direct_path.as_path());
        if direct_classname.is_ok() && *direct_classname.as_ref().unwrap() == name {
            runnerPrint!(runtime, debug, 3, "Name matched for {}", name);
            let maybe_read = read(Path::new(&direct_path));
            if maybe_read.is_ok() {
                return Ok(maybe_read.unwrap());
            }
        }

        if false {
            runnerPrint!(runtime, debug, 3, "Finding class {} direct load failed ({}), searching {}",
                name, match &direct_classname {
                    &Ok(ref x) => x.clone(),
                    &Err(ref y) => format!("{:?}", y),
                }, class_path);

            // Else try globbing
            let mut glob_path = class_path.clone();
            glob_path.push_str("/**/*.class");
            let maybe_glob = glob(glob_path.as_str());
            if maybe_glob.is_err() {
                runnerPrint!(runtime, true, 1, "Error globbing class path {}", class_path);
                continue;
            }

            let class_match = maybe_glob.unwrap()
                .filter_map(Result::ok)
                .filter(|x| {
                    let classname = get_classname(&x);
                    return classname.is_ok() && classname.unwrap() == name;
                })
                .nth(0);

            if class_match.is_none() {
                runnerPrint!(runtime, debug, 2, "Could not find {} on class path {}", name, class_path);
                continue;
            }

            let maybe_read = read(&class_match.unwrap());
            if maybe_read.is_err() {
                runnerPrint!(runtime, true, 1, "Error reading class {} on class path {}", name, class_path);
                continue;
            }

            return Ok(maybe_read.unwrap());
        } else {
            runnerPrint!(runtime, debug, 2, "Could not find {} on class path {} (Error {:?})", name, class_path, direct_classname);
            continue;
        }
    }
    return Err(RunnerError::ClassNotLoaded(String::from(name)));
}

fn load_class(runtime: &mut Runtime, name: &str) -> Result<Rc<Class>, RunnerError> {
    {
        let maybe_class = runtime.classes.get(name).map(|x| x.clone());
        if maybe_class.is_some() {
            let x = maybe_class.unwrap().clone();
            try!(initialise_class_stage_2(runtime, &x));
            return Ok(x);
        }
    }
    runnerPrint!(runtime, true, 2, "Finding class {} not already loaded", name);
    let class_result = try!(find_class(runtime,name));
    let class_obj = try!(bootstrap_class_and_dependencies(runtime, name, &class_result));

    return Ok(class_obj);
}

fn bootstrap_class_and_dependencies(runtime: &mut Runtime, name: &str, class_result: &ClassResult) -> Result<Rc<Class>, RunnerError>  {
    let debug = false;

    let new_class = Rc::new(Class::new(&String::from(name), class_result));
    runtime.classes.insert(String::from(name), new_class.clone());
    runnerPrint!(runtime, debug, 1, "Bootstrapping {}", name);
    try!(initialise_class_stage_1(runtime, new_class.clone()));
    try!(initialise_class_stage_2(runtime, &new_class));
    runnerPrint!(runtime, debug, 1, "Bootstrap totally complete on {}", name);
    return Ok(new_class);
}

fn initialise_class_stage_1(runtime: &mut Runtime, mut class: Rc<Class>) -> Result<(), RunnerError> {
    let debug = false;

    runnerPrint!(runtime, debug, 2, "Initialising class stage 1 {}", class.name);

    // Loop down superclass chain
    while !*class.initialising.borrow() && !*class.initialised.borrow() {
        // Initialise variables, refs can be unresolved
        for field in &class.cr.fields {
            if field.access_flags & ACC_STATIC == 0 {
                continue;
            }

            let name_string = try!(get_cp_str(&class.cr.constant_pool, field.name_index));
            let descriptor_string = try!(get_cp_str(&class.cr.constant_pool, field.descriptor_index));

            runnerPrint!(runtime, debug, 3, "Constructing class static member {} {}", name_string, descriptor_string);

            let var = try!(initialise_variable(runtime, descriptor_string.as_str()));

            runnerPrint!(runtime, debug, 3, "Constructed with {}", var);

            class.statics.borrow_mut().insert((*name_string).clone(), var);
        }

        let super_class_name =
            if class.cr.super_class_index > 0 {
                (*try!(get_cp_class(&class.cr.constant_pool, class.cr.super_class_index))).clone()
            } else if class.name != "java/lang/Object" {
                String::from("java/lang/Object")
            } else {
                return Ok(());
            };

        runnerPrint!(runtime, debug, 3, "Class {} has superclass {}", class.name, super_class_name);
        {
            let maybe_superclass = runtime.classes.get(&super_class_name);
            if maybe_superclass.is_some() {
                *class.super_class.borrow_mut() = Some(maybe_superclass.unwrap().clone());
                return Ok(());
            }
        }

        runnerPrint!(runtime, debug, 2, "Finding super class {} not already loaded", super_class_name);
        let class_result = try!(find_class(runtime, super_class_name.as_str()));
        let new_class = Rc::new(Class::new(&super_class_name, &class_result));
        runtime.classes.insert(super_class_name, new_class.clone());
        *class.super_class.borrow_mut() = Some(new_class.clone());

        class = new_class;
    }

    return Ok(());
}

fn initialise_class_stage_2(runtime: &mut Runtime, class: &Rc<Class>) -> Result<(), RunnerError> {
    let debug = false;

    if *class.initialising.borrow() || *class.initialised.borrow() {
        return Ok(());
    }
    runnerPrint!(runtime, debug, 2, "Initialising class stage 2 {}", class.name);
    *class.initialising.borrow_mut() = true;
    try!(invoke_nested(runtime, class.clone(), Vec::new(), "<clinit>", "()V", true));
    *class.initialised.borrow_mut() = true;
    runnerPrint!(runtime, debug, 2, "Class '{}' stage 2 init complete", class.name);

    return Ok(());
}

fn generate_class_descriptor(class: &Rc<Class>) -> String {
    let mut ret = String::new();
    ret.push('L');
    ret.push_str(class.name.as_str());
    ret.push(';');
    return ret;
}

fn generate_method_descriptor(args: &Vec<Variable>, return_descriptor: String, is_static: bool) -> String {
    let mut ret = String::new();
    ret.push('(');
    for arg in args.iter().skip(if is_static {0} else {1}) {
        ret.push_str(arg.get_descriptor().as_str());
    }
    ret.push(')');
    ret.push_str(return_descriptor.as_str());
    return ret;
}

fn extract_type_info_from_descriptor(runtime: &mut Runtime, string: &str, resolve: bool) -> Result<(Variable, u32), RunnerError> {
    let mut iter = string.chars();

    let mut maybe_type_specifier = iter.next();

    if maybe_type_specifier.is_none() {
        runnerPrint!(runtime, true, 2, "Type specifier blank");
        return Err(RunnerError::ClassInvalid("Type specifier blank"));
    }

    let mut array_depth = 0;
    while maybe_type_specifier.unwrap_or(' ') == '[' {
        array_depth = array_depth + 1;
        maybe_type_specifier = iter.next();
    }

    if maybe_type_specifier.is_none() {
        runnerPrint!(runtime, true, 2, "Type specifier invalid {}", string);
        return Err(RunnerError::ClassInvalid2(format!("Type specifier invalid {}", string)));
    }

    let variable;
    match maybe_type_specifier.unwrap() {
        'B' => variable = Variable::Byte(0),
        'C' => variable = Variable::Char('\0'),
        'D' => variable = Variable::Double(0.0),
        'F' => variable = Variable::Float(0.0),
        'I' => variable = Variable::Int(0),
        'J' => variable = Variable::Long(0),
        'S' => variable = Variable::Short(0),
        'Z' => variable = Variable::Boolean(false),
        _ => {
            let type_string : String =
                if maybe_type_specifier.unwrap() == 'L' {
                    iter.take_while(|x| *x != ';').collect()
                } else {
                    String::from(string)
                };
            if resolve {
                let class = try!(load_class(runtime, type_string.as_str()));
                variable = try!(construct_null_object(runtime, class));
            } else {
                if runtime.classes.contains_key(type_string.as_str()) {
                    let class = runtime.classes.get(type_string.as_str()).unwrap().clone();
                    variable = try!(construct_null_object(runtime, class));
                } else {
                    variable = Variable::UnresolvedReference(type_string.clone());
                }
            }
        }
    }

    return Ok((variable, array_depth));
}

fn parse_single_type_string(runtime: &mut Runtime, type_string: &str, resolve: bool) -> Result<Variable, RunnerError> {
    let (variable, array_depth) = try!(extract_type_info_from_descriptor(runtime, type_string, resolve));

    if array_depth > 0 {
        if array_depth > 1 {
            runnerPrint!(runtime, true, 1, "Warning: >1 array depth, is this right?");
        }
        if variable.is_primitive() {
            return Ok(try!(construct_primitive_array(runtime, variable.get_descriptor().as_str(), None)));
        } else if variable.is_unresolved() {
            return Ok(Variable::UnresolvedReference(String::from(type_string)));
        } else {
            return Ok(try!(construct_array(runtime, variable.to_ref().type_ref.clone(), None)));
        }
    } else {
        return Ok(variable);
    }
}

fn parse_function_type_string(runtime: &mut Runtime, string: &str) -> Result<(Vec<Variable>, Option<Variable>), RunnerError> {
    let debug = false;
    let mut iter = string.chars().peekable();

    if iter.next().unwrap_or(' ') != '(' {
        runnerPrint!(runtime, true, 2, "Type {} invalid", string);
        return Err(RunnerError::ClassInvalid2(format!("Type {} invalid", string)));
    }

    let mut parameters : Vec<Variable> = Vec::new();
    let mut type_char : char;
    while {type_char = try!(iter.next().ok_or(RunnerError::ClassInvalid2(format!("Failed to parse {}", string)))); type_char != ')'} {
        let mut type_string = String::new();
        while type_char == '[' {
            type_string.push(type_char);
            type_char = try!(iter.next().ok_or(RunnerError::ClassInvalid2(format!("Failed to parse {}", string))));
        }
        type_string.push(type_char);

        if type_char == 'L' {
            type_string.push_str(iter.by_ref().take_while(|x| *x != ';').collect::<String>().as_str());
        }
        runnerPrint!(runtime, debug, 3, "Found parameter {}", type_string);
        let param = try!(parse_single_type_string(runtime, type_string.as_str(), true));
        if !param.is_type_1() {
            parameters.push(param.clone());
        }
        parameters.push(param);
        runnerPrint!(runtime, debug, 3, "Parameters now {:?}", parameters);
    }

    let return_type_string : String = iter.collect();
    runnerPrint!(runtime, debug, 3, "Return type {}", return_type_string);
    if return_type_string == "V" {
        return Ok((parameters, None));
    } else {
        return Ok((parameters, Some(try!(parse_single_type_string(runtime, return_type_string.as_str(), true)))));
    }
}

pub fn run(class_paths: &Vec<String>, class: &ClassResult) -> Result<(), RunnerError> {
    println!("Running");
    let mut runtime = Runtime::new(class_paths.clone());
    runtime.current_frame.constant_pool = class.constant_pool.clone();

    try!(bootstrap_class_and_dependencies(&mut runtime, String::new().as_str(), class));

    let main_code = try!(get_class_method_code(class, &"main", &"([Ljava/lang/String;)V"));
    runtime.current_frame.code = main_code;

    try!(do_run_method(&mut runtime));

    return Ok(());
}

pub fn get_runtime(class_paths: &Vec<String>) -> Runtime {
    return Runtime::new(class_paths.clone());
}

pub fn run_method(runtime: &mut Runtime, class_result: &ClassResult, method: &str, arguments: &Vec<Variable>, return_descriptor: String) -> Result<Variable, RunnerError> {
    println!("Running method {} with {} arguments", method, arguments.len());

    runtime.reset_frames();
    runtime.current_frame.constant_pool = class_result.constant_pool.clone();

    let name = try!(class_result.name());
    let class = try!(bootstrap_class_and_dependencies(runtime, name.as_str(), class_result));

    runtime.current_frame.class = Some(class);
    for arg in arguments {
        match arg {
            &Variable::Long(ref _x) => {
                runtime.current_frame.local_variables.push(arg.clone());
                runtime.current_frame.local_variables.push(arg.clone());
            },
            &Variable::Double(ref _x) => {
                runtime.current_frame.local_variables.push(arg.clone());
                runtime.current_frame.local_variables.push(arg.clone());
            },
            _ => {
                runtime.current_frame.local_variables.push(arg.clone());
            }
        }
    }

    let method_descriptor = generate_method_descriptor(&arguments, return_descriptor, true);
    runnerPrint!(runtime, true, 1, "Finding method {} with descriptor {}", method, method_descriptor);
    let code = try!(get_class_method_code(class_result, method, method_descriptor.as_str()));

    println!("Running method");
    runtime.current_frame.code = code;
    try!(do_run_method(runtime));

    return Ok(pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap().clone());
}