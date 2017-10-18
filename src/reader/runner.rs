#![deny(
    non_snake_case,
    unreachable_code,
    unused_assignments,
    unused_imports,
    unused_variables,
    unused_mut,
)]

extern crate byteorder;
use reader::class::*;
use std;
use std::fmt;
use std::io;
use std::io::Cursor;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::Add;
use std::ops::Sub;
use std::ops::Mul;
use std::ops::Div;
use std::ops::Rem;
use std::ops::BitAnd;
use std::ops::BitOr;
use std::ops::BitXor;
use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;
use std::rc::Weak;
use std::path::Path;
use std::path::PathBuf;
use glob::glob;

use self::byteorder::{BigEndian, ReadBytesExt};

#[derive(Debug)]
pub enum RunnerError {
    ClassInvalid(&'static str),
    ClassInvalid2(String),
    InvalidPc,
    IoError,
    NativeMethod(String),
    UnknownOpCode(u8),
    ClassNotLoaded(String),
    NullPointerException,
    ArrayIndexOutOfBoundsException(usize, usize)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Class {
    name: String,
    cr: ClassResult,
    initialising: RefCell<bool>,
    initialised: RefCell<bool>,
    statics: RefCell<HashMap<String, Variable>>,
    super_class: RefCell<Option<Rc<Class>>>
}
impl Class {
  pub fn new(name: &String, cr: &ClassResult) -> Class {
      return Class { name: name.clone(), initialising: RefCell::new(false), initialised: RefCell::new(false), cr: cr.clone(), statics: RefCell::new(HashMap::new()), super_class: RefCell::new(None)};
  }
}

#[derive(Clone, Debug)]
pub struct Object {
    type_ref: Rc<Class>,
    members: RefCell<HashMap<String, Variable>>,
    super_class: RefCell<Option<Rc<Object>>>,
    sub_class: RefCell<Option<Weak<Object>>>
}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return match self.type_ref.name.as_str() {
            "java/lang/String" => {
                let str = string_to_string(self);
                write!(f, "String {}", str.as_str())
            }
            _ => {write!(f, "Object {}", self.type_ref.name.as_str()) }
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
pub enum Variable {
    Byte(u8),
    Char(char),
    Double(f64),
    Float(f32),
    Int(i32),
    Long(i64),
    Short(i16),
    Boolean(bool),
    Reference(Rc<Class>, Option<Rc<Object>>),
    ArrayReference(Rc<Variable>, Option<Rc<RefCell<Vec<Variable>>>>), // First argument is dummy for array type
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
            &Variable::Reference(ref class, ref _obj) => {
                return class.clone();
            },
            &Variable::ArrayReference(ref typee, ref _obj) => {
                return typee.to_ref_type();
            },
            _ => {
                panic!("Couldn't convert to reference");
            }
        }
    }
    pub fn to_ref(&self) -> Option<Rc<Object>> {
        match self {
            &Variable::Reference(ref _class, ref obj) => {
                return obj.clone();
            },
            _ => {
                panic!("Couldn't convert to reference");
            }
        }
    }
    pub fn is_ref_or_array(&self) -> bool {
        match self {
            &Variable::Reference(ref _class, ref _obj) => {
                return true;
            },
            &Variable::ArrayReference(ref _type, ref _array) => {
                return true;
            },
            _ => {
                panic!("Couldn't convert to reference or array");
            }
        }
    }
    pub fn is_null(&self) -> bool {
        match self {
            &Variable::Reference(ref _class, ref obj) => {
                return obj.is_none();
            },
            &Variable::ArrayReference(ref _type, ref array) => {
                return array.is_none();
            },
            _ => {
                panic!("Couldn't check if primitive '{}' is null", self);
            }
        }
    }
    pub fn to_arrayref(&self) -> (Rc<Variable>, &Option<Rc<RefCell<Vec<Variable>>>>) {
        match self {
            &Variable::ArrayReference(ref typee, ref array) => {
                return (typee.clone(), array);
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
}
impl fmt::Display for Variable {
     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
         match self {
             &Variable::Reference(ref class, ref maybe_ref) => {
                 match maybe_ref {
                     &Option::None => {write!(f, "Reference ({} NULL)", class.name)}
                     &Option::Some(ref x) => {write!(f, "Reference ({} {})", class.name, x)}
                 }
             },
             &Variable::ArrayReference(ref vtype, ref maybe_ref) => {
                 if maybe_ref.is_some() {
                     let vec = maybe_ref.as_ref().unwrap().borrow();
                     write!(f, "ArrayReference Size:{} ({})",
                        vec.len(),
                        vec.iter()
                                .take(10)
                                .map(|y| format!("{}", y))
                                .fold(String::new(), |a, b| (a + ", " + b.as_str())))
                 } else {
                     write!(f, "ArrayReference (None) {:?}", vtype)
                 }
             },
             _ => {
                 write!(f, "{:?}", self)
             }
         }
     }
}

#[derive(Clone, Debug)]
struct Frame {
    class: Option<Rc<Class>>,
    constant_pool: HashMap<u16, ConstantPoolItem>,
    local_variables: Vec<Variable>,
    operand_stack: Vec<Variable>,
}

struct Runtime {
    previous_frames: Vec<Frame>,
    current_frame: Frame,
    class_paths: Vec<String>,
    classes: HashMap<String, Rc<Class>>,
    count: i64,
    current_thread: Option<Variable>,
    string_interns: HashMap<String, Variable>,
    properties: HashMap<String, Variable>,
    class_objects: HashMap<String, Variable>,
}
impl Runtime {
    fn  new(class_paths: Vec<String>, constant_pool: HashMap<u16, ConstantPoolItem>) -> Runtime {
        return Runtime {
            class_paths: class_paths,
            previous_frames: vec!(Frame {
                class: None,
                constant_pool: HashMap::new(),
                operand_stack: Vec::new(),
                local_variables: Vec::new()}),
            current_frame: Frame {
                class: None,
                constant_pool: constant_pool,
                operand_stack: Vec::new(),
                local_variables: Vec::new()},
            classes: HashMap::new(),
            count: 0,
            current_thread: None,
            string_interns: HashMap::new(),
            properties: HashMap::new(),
            class_objects: HashMap::new()
        };
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

fn get_cp_str(constant_pool: &HashMap<u16, ConstantPoolItem>, index:u16) -> Result<Rc<String>, RunnerError> {
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
                return Err(RunnerError::ClassInvalid("Error"));
            }
        }
    }
}

fn push_on_stack(operand_stack: &mut Vec<Variable>, var: Variable) {
    if !var.is_type_1() {
        operand_stack.push(var.clone());
    }
    operand_stack.push(var);
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
        return Err(RunnerError::ClassInvalid("Error"));
    } else {
        match *maybe_cp_entry.unwrap() {
            ConstantPoolItem::CONSTANT_Class {index} => {
                debugPrint!(false, 4, "name_index: {}", index);

                let name_str = try!(get_cp_str(&constant_pool, index));
                return Ok(name_str);
            }
            _ => {
                println!("Index {} is not a class", index);

                return Err(RunnerError::ClassInvalid("Error"));
            }
        }
    }
}

fn get_cp_name_and_type(constant_pool: &HashMap<u16, ConstantPoolItem>, index: u16) -> Result<(Rc<String>, Rc<String>), RunnerError> {
    debugPrint!(false, 5, "{}", index);

    let maybe_cp_entry = constant_pool.get(&index);
    if maybe_cp_entry.is_none() {
        debugPrint!(true, 1, "Missing CP name & type {}", index);
        return Err(RunnerError::ClassInvalid("Error"));
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

                return Err(RunnerError::ClassInvalid("Error"));
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
        return Err(RunnerError::ClassInvalid("Error"));
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
                println!("Index {} is not a method", index);
                return Err(RunnerError::ClassInvalid("Error"));
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

fn construct_object(runtime: &mut Runtime, name: &str) -> Result<Variable, RunnerError> {
    let debug = false;
    debugPrint!(debug, 3, "Constructing object {}", name);
    try!(load_class(runtime, name));

    let original_class = try!(runtime.classes.get(name).ok_or(RunnerError::ClassInvalid2(format!("Failed to find class {}", name)))).clone();
    let mut original_obj : Option<Rc<Object>> = None;
    let mut class = original_class.clone();
    let mut sub_class : Option<Weak<Object>> = None;

    loop {
        debugPrint!(debug, 3, "Constructing object of type {} with subclass {}", class.name, sub_class.is_some());
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

        let obj = Rc::new(Object { type_ref: class.clone(), members: RefCell::new(members), super_class: RefCell::new(None), sub_class: RefCell::new(sub_class.clone()) });
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
            return Ok(Variable::Reference(original_class.clone(), original_obj));
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

fn get_obj_instance_from_variable(var: &Variable) -> Result<Option<Rc<Object>>, RunnerError> {
    match var {
        &Variable::Reference(ref _class, ref objref) => {
            return Ok(objref.clone());
        },
        _ => {
            return Err(RunnerError::ClassInvalid("Error"));
        }
    }
}

fn extract_from_char_array(var: &Variable) -> String {
    let (_clazz, array) = var.to_arrayref();
    let mut res = String::new();
    for c in array.as_ref().unwrap().borrow().iter() {
        res.push(c.to_char());
    }
    return res;
}

fn extract_from_string(obj: &Rc<Object>) -> Result<String, RunnerError> {
    let field = try!(get_field(obj, "java/lang/String", "value"));
    let string = extract_from_char_array(&field);
    return Ok(string);
}

fn construct_char_array(s: &str) -> Variable {
    let mut v : Vec<Variable> = Vec::new();
    for c in s.chars() {
        v.push(Variable::Char(c));
    }
    return Variable::ArrayReference(Rc::new(Variable::Char('\0')), Some(Rc::new(RefCell::new(v))));
}

fn string_to_string(obj: &Object) -> String {
    let members = obj.members.borrow();
    let value_array = members.get(&String::from("value"));
    if value_array.is_none() { return String::from("");}
    let (_array_type, maybe_array) = value_array.unwrap().to_arrayref();
    if maybe_array.is_none() { return String::from("");}
    let vec = maybe_array.as_ref().unwrap().borrow();
    let mut ret = String::new();
    for v in vec.iter() {
        ret.push(v.to_char());
    }

    return ret;
}

fn load<F>(desc: &str, index: u8, runtime: &mut Runtime, _t: F) -> Result<(), RunnerError> { // TODO: Type checking
    let loaded = runtime.current_frame.local_variables[index as usize].clone();
    debugPrint!(true, 2, "{} {} {}", desc, index, loaded);
    push_on_stack(&mut runtime.current_frame.operand_stack, loaded);
    return Ok(());
}

fn aload<F, G>(desc: &str, runtime: &mut Runtime, _t: F, converter: G) -> Result<(), RunnerError>
    where G: Fn(Variable) -> Variable
{ // TODO: Type checking
    let index = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap().to_int();
    let var = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    let (_array_type, maybe_array) = var.to_arrayref();
    debugPrint!(true, 2, "{} {} {}", desc, index, var);
    if maybe_array.is_none() {
        return Err(RunnerError::NullPointerException);
    }

    let array = maybe_array.as_ref().unwrap().borrow();
    if array.len() < index as usize {
        return Err(RunnerError::ArrayIndexOutOfBoundsException(array.len(), index as usize));
    }

    let item = converter(array[index as usize].clone());

    push_on_stack(&mut runtime.current_frame.operand_stack, item);
    return Ok(());
}

fn store<F>(desc: &str, index: u8, runtime: &mut Runtime, _t: F) -> Result<(), RunnerError> { // TODO: Type checking
    let popped = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    debugPrint!(true, 2, "{}_{} {}", desc, index, popped);
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
    let (_array_type, maybe_array) = var.to_arrayref();
    debugPrint!(true, 2, "{} {} {}", desc, index, var);
    if maybe_array.is_none() {
        return Err(RunnerError::NullPointerException);
    }

    let mut array = maybe_array.as_ref().unwrap().borrow_mut();
    if array.len() < index as usize {
        return Err(RunnerError::ArrayIndexOutOfBoundsException(array.len(), index as usize));
    }

    array[index as usize] = converter(&value);
    return Ok(());
}

// TODO: Overflow checks
fn add<F>(a: F, b: F) -> <F as std::ops::Add>::Output where F: Add { a+b }
fn sub<F>(a: F, b: F) -> <F as std::ops::Sub>::Output where F: Sub { b-a }
fn mul<F>(a: F, b: F) -> <F as std::ops::Mul>::Output where F: Mul { a*b }
fn div<F>(a: F, b: F) -> <F as std::ops::Div>::Output where F: Div { b/a }
fn rem<F>(a: F, b: F) -> <F as std::ops::Rem>::Output where F: Rem { b%a }
fn and<F>(a: F, b: F) -> <F as std::ops::BitAnd>::Output where F: BitAnd { b&a }
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
    debugPrint!(true, 2, "{} {} {}", desc, popped1, popped2);
    push_on_stack(&mut runtime.current_frame.operand_stack, creator(operation(extractor(&popped1), extractor(&popped2))));
}

fn maths_instr_2<F, G, H, I, J, K, L>(desc: &str, runtime: &mut Runtime, creator: F, extractor1: G, extractor2: H, operation: I)
    where
        F: Fn(L) -> Variable,
        G: Fn(&Variable) -> J,
        H: Fn(&Variable) -> K,
        I: Fn(J, K) -> L
{
    let popped1 = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    let popped2 = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    debugPrint!(true, 2, "{} {} {}", desc, popped1, popped2);
    push_on_stack(&mut runtime.current_frame.operand_stack, creator(operation(extractor1(&popped1), extractor2(&popped2))));
}

fn single_pop_instr<F, G, H, I, J>(desc: &str, runtime: &mut Runtime, creator: F, extractor: G, operation: H)
    where
    F: Fn(J) -> Variable,
    G: Fn(&Variable) -> I,
    H: Fn(I) -> J
{
    let popped = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    debugPrint!(true, 2, "{} {}", desc, popped);
    push_on_stack(&mut runtime.current_frame.operand_stack, creator(operation(extractor(&popped))));
}

fn vreturn<F, K>(desc: &str, runtime: &mut Runtime, extractor: F) -> Result<(), RunnerError> where F: Fn(&Variable) -> K {
    let popped = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    debugPrint!(true, 1, "{} {}", desc, popped);
    extractor(&popped); // Type check
    runtime.current_frame = runtime.previous_frames.pop().unwrap();
    push_on_stack(&mut runtime.current_frame.operand_stack, popped);
    return Ok(());
}

// Get the (super)object which contains a field
fn get_obj_field(mut obj: Rc<Object>, field_name: &str) -> Result<Rc<Object>, RunnerError> {
    let class_name = obj.type_ref.name.clone();
    while {let members = obj.members.borrow(); !members.contains_key(field_name) } {
        let new_obj = obj.super_class.borrow().clone();
        if new_obj.is_none() {
            return Err(RunnerError::ClassInvalid2(format!("Couldn't find field {} in class {}", field_name, class_name)));
        }
        obj = new_obj.unwrap();
    }
    return Ok(obj.clone());
}

fn get_super_obj(mut obj: Rc<Object>, class_name: &str) -> Result<Rc<Object>, RunnerError> {
    while obj.type_ref.name != class_name && obj.super_class.borrow().is_some() {
        let new_obj = obj.super_class.borrow().clone().unwrap();
        obj = new_obj;
        debugPrint!(false, 3, "Class didn't match, checking {} now)", obj.type_ref.name);
    }

    if obj.type_ref.name != class_name {
        debugPrint!(true, 1, "Expected object on stack with class name {} but got {}", class_name, obj.type_ref.name);
        return Err(RunnerError::ClassInvalid2(format!("Couldn't find object on stack with class name {}", class_name)));
    }

    return Ok(obj);
}

fn invoke_manual(runtime: &mut Runtime, class: Rc<Class>, args: Vec<Variable>, method_name: &str, method_descriptor: &str, allow_not_found: bool) -> Result<(), RunnerError>{
    let new_frame = Frame {
        class: Some(class.clone()),
        constant_pool: class.cr.constant_pool.clone(),
        operand_stack: Vec::new(),
        local_variables: args.clone()};

    let maybe_code = get_class_method_code(&class.cr, method_name, method_descriptor);
    if maybe_code.is_err() {
        if allow_not_found { return Ok(()) }
        else { return Err(maybe_code.err().unwrap()) };
    }
    let code = maybe_code.unwrap();

    debugPrint!(true, 3, "Invoking manually {} {} on {}", method_name, method_descriptor, class.name);
    runtime.previous_frames.push(runtime.current_frame.clone());
    runtime.current_frame = new_frame;
    try!(do_run_method((class.name.clone() + method_name).as_str(), runtime, &code, 0));

    return Ok(());
}

fn hash_var<H>(var: &Variable, state: &mut H) where H: Hasher {
    match var {
        &Variable::Boolean(ref x) => {x.hash(state);}
        &Variable::Byte(ref x) => {x.hash(state);}
        &Variable::Char(ref x) => {x.hash(state);}
        &Variable::Short(ref x) => {x.hash(state);}
        &Variable::Int(ref x) => {x.hash(state);}
        &Variable::Long(ref x) => {x.hash(state);}
        &Variable::Float(ref x) => { unsafe {std::mem::transmute::<f32, u32>(*x)}.hash(state);}
        &Variable::Double(ref x) => { unsafe {std::mem::transmute::<f64, u64>(*x)}.hash(state);}
        &Variable::Reference(ref _class, ref obj) => { obj.as_ref().map(|x| hash_obj(x.clone(), state)); }
        &Variable::ArrayReference(ref _type, ref array) => {
            array.as_ref().map(|x| for y in x.borrow().iter() { hash_var(y, state); });
        }
        &Variable::InterfaceReference(ref x) => { hash_obj(x.clone(), state); }
        &Variable::UnresolvedReference(ref x) => { x.hash(state); }
    }
}

fn hash_obj<H>(obj: Rc<Object>, state: &mut H) where H: Hasher {
    let mut mobj = Some(get_most_sub_class(obj));
    while mobj.is_some() {
        let aobj = mobj.unwrap();
        let members = aobj.members.borrow();
        for (_key, value) in members.iter() {
            hash_var(value, state);
        }

        let new_obj = aobj.super_class.borrow().clone();
        mobj = new_obj;
    }
}


fn string_intern(runtime: &mut Runtime, var: &Variable) -> Result<Variable, RunnerError> {
    let obj = var.to_ref().unwrap();
    let string = try!(extract_from_string(&obj));
    if !runtime.string_interns.contains_key(&string) {
        runtime.string_interns.insert(string.clone(), var.clone());
    }
    return Ok(runtime.string_interns.get(&string).unwrap().clone());
}

fn try_builtin(class_name: &Rc<String>, method_name: &Rc<String>, descriptor: &Rc<String>, args: &Vec<Variable>, runtime: &mut Runtime) -> Result<bool, RunnerError> {
    debugPrint!(true, 1, "try_builtin {} {} {}", class_name, method_name, descriptor);
    match (class_name.as_str(), method_name.as_str(), descriptor.as_str()) {
        ("java/net/InetAddress", "init", "()V") => {}
        ("java/net/InetAddressImplFactory", "isIPv6Supported", "()Z") => {push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Boolean(false));}
        ("java/util/concurrent/atomic/AtomicLong", "VMSupportsCS8", "()Z") => {push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Boolean(false));}
        ("java/lang/Class", "registerNatives", "()V") => {}
        ("java/lang/Class", "isArray", "()Z") => {
            let obj = args[0].clone().to_ref().unwrap();
            let members = obj.members.borrow();
            let value = members.get(&String::from("__is_array")).unwrap();
            debugPrint!(true, 2, "BUILTIN: is_array {}", value);
            push_on_stack(&mut runtime.current_frame.operand_stack, value.clone());
        }
        ("java/lang/Class", "isPrimitive", "()Z") => {
            let obj = args[0].clone().to_ref().unwrap();
            let members = obj.members.borrow();
            let value = members.get(&String::from("__is_primitive")).unwrap();
            debugPrint!(true, 2, "BUILTIN: is_primitive {}", value);
            push_on_stack(&mut runtime.current_frame.operand_stack, value.clone());
        }
        ("java/lang/Class", "getPrimitiveClass", "(Ljava/lang/String;)Ljava/lang/Class;") => {
            let obj = args[0].clone().to_ref().unwrap();
            let string = try!(extract_from_string(&obj));
            debugPrint!(true, 2, "BUILTIN: getPrimitiveClass {}", string);
            let var = try!(get_primitive_class(runtime, string));
            push_on_stack(&mut runtime.current_frame.operand_stack, var);
        }
        ("java/lang/Class", "isAssignableFrom", "(Ljava/lang/Class;)Z") => {
            let class_object_1 = args[0].clone().to_ref().unwrap();
            let mut class1 = class_object_1.members.borrow().get(&String::from("__class")).unwrap().to_ref_type();
            let class_object_2 = args[1].clone().to_ref().unwrap();
            let class2 = class_object_2.members.borrow().get(&String::from("__class")).unwrap().to_ref_type();
            while class1 != class2 {
                if class1.super_class.borrow().is_none() { break; }
                let new_class1 = class1.super_class.borrow().clone().unwrap();
                class1 = new_class1;
            }

            push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Boolean(class1 == class2));
        }
        ("java/lang/Class", "getComponentType", "()Ljava/lang/Class;") => {
            let class_object_1 = args[0].clone().to_ref().unwrap();
            let is_array = class_object_1.members.borrow().get(&String::from("__is_array")).unwrap().to_bool();
            let is_primitive = class_object_1.members.borrow().get(&String::from("__is_primitive")).unwrap().to_bool();
            let var =
                if is_array {
                    if is_primitive {
                        args[0].clone() // TODO: this is rubbish
                    } else {
                        let component_class_descriptor = generate_variable_descriptor(class_object_1.members.borrow().get(&String::from("__class")).unwrap());
                        try!(make_class(runtime, component_class_descriptor.as_str()))
                    }
                } else {
                    try!(construct_object(runtime, &"java/lang/Class"))
                };
            debugPrint!(true, 2, "BUILTIN: getComponentType {}", var);

            push_on_stack(&mut runtime.current_frame.operand_stack, var);
        },
        ("java/lang/Class", "forName0", "(Ljava/lang/String;ZLjava/lang/ClassLoader;Ljava/lang/Class;)Ljava/lang/Class;") => {
            let descriptor_string_obj = args[0].clone().to_ref().unwrap();
            let descriptor = try!(extract_from_string(&descriptor_string_obj));
            let initialize = args[1].to_bool();
            let ref class_loader = args[2];
            let ref caller_class = args[3];
            debugPrint!(true, 2, "BUILTIN: forName0 {} {} {} {}", descriptor, initialize, class_loader, caller_class);
            let var = try!(make_class(runtime, descriptor.as_str()));
            push_on_stack(&mut runtime.current_frame.operand_stack, var);
        }
        ("java/lang/Class", "desiredAssertionStatus0", "(Ljava/lang/Class;)Z") => {push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Boolean(false));}
        ("java/lang/System", "arraycopy", "(Ljava/lang/Object;ILjava/lang/Object;II)V") => {
            debugPrint!(true, 2, "BUILTIN: arrayCopy {} {} {} {} {}", args[0], args[1], args[2], args[3], args[4]);

            let (_x, src) = args[0].to_arrayref();
            let src_pos = args[1].to_int();
            let (_x, dest) = args[2].to_arrayref();
            let dest_pos = args[3].to_int();
            let length = args[4].to_int();

            if src.is_none() || dest.is_none() {
                // TODO
                return Err(RunnerError::NullPointerException);
            }

            let src_data = src.as_ref().unwrap().borrow();
            let mut dest_data = dest.as_ref().unwrap().borrow_mut();

            for n in 0..length {
                dest_data[(dest_pos + n) as usize] = src_data[(src_pos + n) as usize].clone();
            }
        },
        ("java/lang/System", "registerNatives", "()V") => {},
        ("java/lang/System", "loadLibrary", "(Ljava/lang/String;)V") => {
            let lib_string_obj = args[0].clone().to_ref().unwrap();
            let lib = try!(extract_from_string(&lib_string_obj));
            debugPrint!(true, 2, "BUILTIN: loadLibrary {}", lib);
        }
        ("java/lang/System", "getProperty", "(Ljava/lang/String;)Ljava/lang/String;") => {
            let obj = args[0].clone().to_ref().unwrap();
            let string = try!(extract_from_string(&obj));
            if runtime.properties.contains_key(&string) {
                debugPrint!(true, 2, "BUILTIN: getProperty {} valid", string);
                push_on_stack(&mut runtime.current_frame.operand_stack, runtime.properties.get(&string).unwrap().clone());
            } else {
                debugPrint!(true, 2, "BUILTIN: getProperty {} NULL", string);
                let null_string = Variable::Reference(try!(load_class(runtime, "java/lang/String")), None);
                push_on_stack(&mut runtime.current_frame.operand_stack, null_string);
            }
        },
        ("java/lang/Runtime", "availableProcessors", "()I") => {
            debugPrint!(true, 2, "BUILTIN: availableProcessors");
            push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Int(1));
        },
        ("java/lang/Object", "registerNatives", "()V") => {return Ok(true)},
        ("sun/misc/Unsafe", "registerNatives", "()V") => {return Ok(true)},
        ("sun/misc/Unsafe", "arrayBaseOffset", "(Ljava/lang/Class;)I") => {
            debugPrint!(true, 2, "BUILTIN: arrayBaseOffset");
            push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Int(0));
        },
        ("sun/misc/Unsafe", "objectFieldOffset", "(Ljava/lang/reflect/Field;)J") => {
            debugPrint!(true, 2, "BUILTIN: objectFieldOffset");
            push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Long(0)); // TODO: this is rubbish
        },
        ("sun/misc/Unsafe", "arrayIndexScale", "(Ljava/lang/Class;)I") => {
            debugPrint!(true, 2, "BUILTIN: arrayIndexScale");
            push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Int(1));
        },
        ("sun/misc/Unsafe", "addressSize", "()I") => {
            debugPrint!(true, 2, "BUILTIN: addressSize");
            push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Int(4));
        },
        ("sun/misc/Unsafe", "pageSize", "()I") => {
            debugPrint!(true, 2, "BUILTIN: pageSize");
            push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Int(4096));
        },
        ("java/lang/String", "intern", "()Ljava/lang/String;") => {
            let interned = try!(string_intern(runtime, &args[0]));
            debugPrint!(true, 2, "BUILTIN: intern {} {:p}", args[0], &*interned.to_ref().unwrap());
            push_on_stack(&mut runtime.current_frame.operand_stack, interned);
        },
        ("java/lang/Float", "floatToRawIntBits", "(F)I") => {
            let float = args[0].to_float();
            let bits = unsafe {std::mem::transmute::<f32, u32>(float)};
            debugPrint!(true, 2, "BUILTIN: floatToRawIntBits {} {}", float, bits);
            push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Int(bits as i32));
        },
        ("java/lang/Float", "intBitsToFloat", "(I)F") => {
            let int = args[0].to_int();
            let float = unsafe {std::mem::transmute::<i32, f32>(int)};
            debugPrint!(true, 2, "BUILTIN: intBitsToFloat {} {}", int, float);
            push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Float(float));
        },
        ("java/lang/Double", "doubleToRawLongBits", "(D)J") => {
            let double = args[0].to_double();
            let bits = unsafe {std::mem::transmute::<f64, u64>(double)};
            debugPrint!(true, 2, "BUILTIN: doubleToRawIntBits {} {}", double, bits);
            push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Long(bits as i64));
        },
        ("java/lang/Double", "longBitsToDouble", "(J)D") => {
            let long = args[0].to_long();
            let double = unsafe {std::mem::transmute::<i64, f64>(long)};
            debugPrint!(true, 2, "BUILTIN: doubleToRawIntBits {} {}", long, double);
            push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Double(double));
        },
        ("java/lang/SecurityManager", "checkPermission", "(Ljava/security/Permission;)V") => {
        },
        ("java/security/AccessController", "doPrivileged", "(Ljava/security/PrivilegedAction;)Ljava/lang/Object;") => {
            let action = args[0].clone().to_ref().unwrap();
            debugPrint!(true, 2, "BUILTIN: doPrivileged {}", action);
            try!(invoke_manual(runtime, action.type_ref.clone(), args.clone(), "run", "()Ljava/lang/Object;", false));
        },
        ("java/security/AccessController", "getStackAccessControlContext", "()Ljava/security/AccessControlContext;") => {
            let ret = Variable::Reference(try!(load_class(runtime, &"java/security/AccessControlContext")), None);
            push_on_stack(&mut runtime.current_frame.operand_stack, ret);
        }
        ("java/lang/Object", "hashCode", "()I") => {
            let obj = args[0].clone().to_ref().unwrap();
            let mut s = DefaultHasher::new();
            hash_obj(obj, &mut s);
            let hash = s.finish();
            debugPrint!(true, 2, "BUILTIN: hashcode {}", hash);
            push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Int(hash as i32));
        },
        ("java/lang/System", "identityHashCode", "(Ljava/lang/Object;)I") => {
            let obj = args[0].clone().to_ref().unwrap();
            let mut s = DefaultHasher::new();
            hash_obj(obj, &mut s);
            let hash = s.finish();
            debugPrint!(true, 2, "BUILTIN: identityHashCode {}", hash);
            push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Int(hash as i32));
        },
        ("java/lang/Object", "getClass", "()Ljava/lang/Class;") => {
            let ref descriptor = generate_variable_descriptor(&args[0]);
            let var = try!(make_class(runtime, descriptor.as_str()));
            debugPrint!(true, 2, "BUILTIN: getClass {} {}", descriptor, var);
            push_on_stack(&mut runtime.current_frame.operand_stack, var);
        },
        ("java/lang/ClassLoader", "registerNatives", "()V") => {},
        ("java/lang/Thread", "registerNatives", "()V") => {},
        ("java/lang/Thread", "isAlive", "()Z") => {
            let obj = args[0].clone().to_ref().unwrap();
            let members = obj.members.borrow();
            let var = members.get(&String::from("__alive")).unwrap_or(&Variable::Boolean(false)).clone();
            debugPrint!(true, 2, "BUILTIN: isAlive {}", var);
            push_on_stack(&mut runtime.current_frame.operand_stack, var);
        },
        ("java/lang/Thread", "start0", "()V") => {
            // TODO
        }
        ("java/lang/Thread", "setPriority0", "(I)V") => {
            let obj = args[0].clone().to_ref().unwrap();
            debugPrint!(true, 2, "BUILTIN: setPriority0 {} {}", args[0], args[1]);
            try!(put_field(obj.clone(), &"java/lang/Thread", &"priority", args[1].clone()));
        }
        ("java/lang/Thread", "currentThread", "()Ljava/lang/Thread;") => {
            debugPrint!(true, 2, "BUILTIN: currentThread");
            if runtime.current_thread.is_none() {
                debugPrint!(true, 2, "BUILTIN: currentThread - creating thread");
                let thread_group;
                {
                    let var = try!(construct_object(runtime, &"java/lang/ThreadGroup"));
                    let obj = try!(var.to_ref().ok_or(RunnerError::NullPointerException));
                    try!(invoke_manual(runtime, obj.type_ref.clone(), vec!(var.clone()), "<init>", "()V", false));
                    thread_group = var.clone();
                }

                {
                    let var = try!(construct_object(runtime, &"java/lang/Thread"));

                    runtime.current_thread = Some(var.clone());
                    let obj = try!(var.to_ref().ok_or(RunnerError::NullPointerException));
                    let mut members = obj.members.borrow_mut();
                    members.insert(String::from("name"), try!(make_string(runtime, &"thread")));
                    members.insert(String::from("priority"), Variable::Int(1));
                    members.insert(String::from("group"), thread_group);
                    members.insert(String::from("__alive"), Variable::Boolean(true));
                }
            }
            push_on_stack(&mut runtime.current_frame.operand_stack, runtime.current_thread.as_ref().unwrap().clone());
        }
        ("sun/misc/VM", "initialize", "()V") => {}
        ("sun/reflect/Reflection", "getCallerClass", "()Ljava/lang/Class;") => {
            let class = runtime.previous_frames[runtime.previous_frames.len()-1].class.clone().unwrap();
            let descriptor = String::from("L") + class.name.as_str() + ";";
            let var = try!(make_class(runtime, descriptor.as_str()));
            debugPrint!(true, 2, "BUILTIN: getCallerClass {}", var);
            push_on_stack(&mut runtime.current_frame.operand_stack, var);
        }
        _ => return Ok(false)
    };
    return Ok(true);
}


fn invoke(desc: &str, runtime: &mut Runtime, index: u16, with_obj: bool, special: bool) -> Result<(), RunnerError> {
    let debug = false;
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

        debugPrint!(debug, 1, "{} {} {} {}", desc, class_name, method_name, descriptor);

        if try!(try_builtin(&class_name, &method_name, &descriptor, &new_local_variables, runtime)) {
            return Ok(());
        }

        let mut class = try!(load_class(runtime, class_name.as_str()));

        if with_obj {
            let mut obj = try!(new_local_variables[0].to_ref().ok_or(RunnerError::ClassInvalid2(format!("Missing obj ref on local var stack for method on {}", class_name))));

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
                    return Err(RunnerError::ClassInvalid2(format!("Could not find super class of object that matched method {} {}", method_name, descriptor)))
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
            local_variables: new_local_variables
        });

    }

    runtime.previous_frames.push(runtime.current_frame.clone());
    runtime.current_frame = new_frame.unwrap();
    try!(do_run_method(new_method_name.unwrap().as_str(), runtime, &code.unwrap(), 0));
    return Ok(());
}

fn fcmp(desc: &str, runtime: &mut Runtime, is_g: bool) -> Result<(), RunnerError> {
    let pop2 = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap().to_float();
    let pop1 = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap().to_float();
    debugPrint!(true, 2, "{} {} {}", desc, pop1, pop2);
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
    push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Int(ret));
    return Ok(());
}

fn ifcmp<F>(desc: &str, runtime: &mut Runtime, buf: &mut Cursor<&Vec<u8>>, cmp: F) -> Result<(), RunnerError>
    where F: Fn(i32) -> bool
{
    let current_position = buf.position() - 1;
    let branch_offset = try!(buf.read_u16::<BigEndian>()) as i16;
    let popped = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    debugPrint!(true, 2, "{} {} {}", desc, popped, branch_offset);
    if cmp(popped.to_int()) {
        let new_position = (current_position as i64 + branch_offset as i64) as u64;
        debugPrint!(true, 2, "BRANCHED from {} to {}", current_position, new_position);
        buf.set_position(new_position);
    }
    return Ok(());
}

fn branch_if<F>(desc: &str, runtime: &mut Runtime, buf: &mut Cursor<&Vec<u8>>, current_position: u64, cmp: F) -> Result<(), RunnerError>
    where F: Fn(Variable) -> bool
{
    let branch_offset = try!(buf.read_u16::<BigEndian>()) as i16;
    let var = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    debugPrint!(true, 2, "{} {} {}", desc, var, branch_offset);
    if cmp(var) {
        let new_pos = (current_position as i64 + branch_offset as i64) as u64;
        debugPrint!(true, 2, "BRANCHED from {} to {}", current_position, new_pos);
        buf.set_position(new_pos);
    }
    return Ok(());
}

fn make_string(runtime: &mut Runtime, val: &str) -> Result<Variable, RunnerError> {
    let var = try!(construct_object(runtime, &"java/lang/String"));
    let obj = try!(var.to_ref().ok_or(RunnerError::NullPointerException));
    try!(put_field(obj, &"java/lang/String", &"value", construct_char_array(val)));
    return Ok(var);
}

fn make_field(runtime: &mut Runtime, name: Rc<String>, descriptor: Rc<String>, _access: u16)  -> Result<Variable, RunnerError> {
    let class_name = "java/lang/reflect/Field";
    let name_var = try!(make_string(runtime, name.as_str()));
    let name_var_interned = try!(string_intern(runtime, &name_var));
    let signature_var = try!(make_string(runtime, descriptor.as_str()));
    let var = try!(construct_object(runtime, class_name));
    try!(put_field(var.to_ref().unwrap(), class_name, "name", name_var_interned));
    try!(put_field(var.to_ref().unwrap(), class_name, "signature", signature_var));
    let type_obj = try!(make_class(runtime, descriptor.as_str()));
    try!(put_field(var.to_ref().unwrap(), class_name, "type", type_obj));
    return Ok(var);
}

fn make_method(runtime: &mut Runtime, name: Rc<String>, descriptor: Rc<String>, _access: u16)  -> Result<Variable, RunnerError> {
    let class_name = &"java/lang/reflect/Method";
    let name_var = try!(make_string(runtime, name.as_str()));
    let name_var_interned = try!(string_intern(runtime, &name_var));
    let signature_var = try!(make_string(runtime, descriptor.as_str()));
    let var = try!(construct_object(runtime, class_name));
    try!(put_field(var.to_ref().unwrap(), class_name, "name", name_var_interned));
    try!(put_field(var.to_ref().unwrap(), class_name, "signature", signature_var));
    return Ok(var);
}

fn get_primitive_class(runtime: &mut Runtime, typ: String) -> Result<Variable, RunnerError> {
    {
        let maybe_existing = runtime.class_objects.get(&typ);
        if maybe_existing.is_some() {
            return Ok(maybe_existing.unwrap().clone());
        }
    }

    let var = try!(construct_object(runtime, &"java/lang/Class"));
    try!(put_static(runtime, &"java/lang/Class", &"initted", Variable::Boolean(true)));
    let members = &var.to_ref().unwrap().members;
    members.borrow_mut().insert(String::from("__is_primitive"), Variable::Boolean(true));
    members.borrow_mut().insert(String::from("__is_array"), Variable::Boolean(false));
    runtime.class_objects.insert(typ, var.clone());

    return Ok(var);
}

fn make_class(runtime: &mut Runtime, descriptor: &str) -> Result<Variable, RunnerError> {
    try!(load_class(runtime, &"java/lang/Class"));
    {
        let maybe_existing = runtime.class_objects.get(&String::from(descriptor));
        if maybe_existing.is_some() {
            return Ok(maybe_existing.unwrap().clone());
        }
    }

    let var = try!(construct_object(runtime, &"java/lang/Class"));

    runtime.class_objects.insert(String::from(descriptor), var.clone());

    let name_object = try!(make_string(runtime, descriptor));
    try!(put_field(var.to_ref().unwrap(), &"java/lang/Class", "name", try!(string_intern(runtime, &name_object))));
    try!(put_static(runtime, &"java/lang/Class", &"initted", Variable::Boolean(true)));
    let members = &var.to_ref().unwrap().members;

    let subtype = try!(parse_single_type_string(runtime, descriptor, true));
    let mut is_primitive = false;
    let mut is_array = false;
    match subtype {
        Variable::Reference(class, _) => {
            let reflection_data_object = try!(construct_object(runtime, &"java/lang/Class$ReflectionData"));
            {
                let mut field_objects : Vec<Variable> = Vec::new();
                for field in &class.cr.fields {
                    let name_string = try!(get_cp_str(&class.cr.constant_pool, field.name_index));
                    let descriptor_string = try!(get_cp_str(&class.cr.constant_pool, field.descriptor_index));
                    let field_object = try!(make_field(runtime, name_string, descriptor_string, field.access_flags));
                    field_objects.push(field_object);
                }
                let declared_fields_array = Variable::ArrayReference(Rc::new(try!(construct_object(runtime, &"java/lang/reflect/Field"))),
                                                                   Some(Rc::new(RefCell::new(field_objects))));
                try!(put_field(reflection_data_object.to_ref().unwrap(), &"java/lang/Class$ReflectionData", "declaredFields", declared_fields_array));
            }
            {
                let mut method_objects : Vec<Variable> = Vec::new();
                for method in &class.cr.methods {
                    let name_string = try!(get_cp_str(&class.cr.constant_pool, method.name_index));
                    let descriptor_string = try!(get_cp_str(&class.cr.constant_pool, method.descriptor_index));
                    let methods_object = try!(make_method(runtime, name_string, descriptor_string, method.access_flags));
                    method_objects.push(methods_object);
                }
                let declared_methods_array = Variable::ArrayReference(Rc::new(try!(construct_object(runtime, &"java/lang/reflect/Method"))),
                                                                   Some(Rc::new(RefCell::new(method_objects))));
                try!(put_field(reflection_data_object.to_ref().unwrap(), &"java/lang/Class$ReflectionData", "declaredMethods", declared_methods_array));
            }

            let soft_reference_object = try!(construct_object(runtime, &"java/lang/ref/SoftReference"));
            try!(put_field(soft_reference_object.to_ref().unwrap(), &"java/lang/ref/SoftReference", "referent", reflection_data_object));

            try!(put_field(var.to_ref().unwrap(), &"java/lang/Class", "reflectionData", soft_reference_object));
            members.borrow_mut().insert(String::from("__class"), Variable::Reference(class.clone(), None));
        },
        Variable::ArrayReference(basis, _x) => {
            is_array = true;
            match &*basis {
                &Variable::Reference(ref class, _) => {
                    members.borrow_mut().insert(String::from("__class"), Variable::Reference(class.clone(), None));
                },
                _ => {
                    is_primitive = true;
                }
            }
        },
        _ => { is_primitive = true; }
    }
    members.borrow_mut().insert(String::from("__is_primitive"), Variable::Boolean(is_primitive));
    members.borrow_mut().insert(String::from("__is_array"), Variable::Boolean(is_array));

    return Ok(var);
}

fn put_static(runtime: &mut Runtime, class_name: &str, field_name: &str, value: Variable) -> Result<(), RunnerError> {
    let class_result = try!(load_class(runtime, class_name));
    let mut statics = class_result.statics.borrow_mut();
    if !statics.contains_key(field_name) {
        return Err(RunnerError::ClassNotLoaded(String::from(class_name)));
    }
    statics.insert(String::from(field_name), value);
    return Ok(());
}

fn put_field(obj: Rc<Object>, class_name: &str, field_name: &str, value: Variable) -> Result<(), RunnerError> {
    let super_obj = try!(get_super_obj(obj, class_name));
    let super_obj_with_field = try!(get_obj_field(super_obj, field_name));
    let mut members = super_obj_with_field.members.borrow_mut();
    members.insert(String::from(field_name), value);
    return Ok(());
}

fn get_field(obj: &Rc<Object>, class_name: &str, field_name: &str) -> Result<Variable, RunnerError> {
    let super_obj = try!(get_super_obj(obj.clone(), class_name));
    let super_obj_with_field = try!(get_obj_field(super_obj, field_name));
    let members = super_obj_with_field.members.borrow();
    return Ok(members.get(&*field_name).unwrap().clone());
}

fn icmp<F>(desc: &str, runtime: &mut Runtime, buf: &mut Cursor<&Vec<u8>>, cmp: F) -> Result<(), RunnerError>
    where F: Fn(i32, i32) -> bool
{
    let current_position = buf.position() - 1;
    let branch_offset = try!(buf.read_u16::<BigEndian>()) as i16;
    let popped2 = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    let popped1 = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    debugPrint!(true, 2, "{} {} {} {}", desc, popped1, popped2, branch_offset);
    if cmp(popped1.to_int(), popped2.to_int()) {
        let new_position = (current_position as i64 + branch_offset as i64) as u64;
        debugPrint!(true, 2, "BRANCHED from {} to {}", current_position, new_position);
        buf.set_position(new_position);
    }
    return Ok(());
}

fn rc_ptr_eq<T: ?Sized>(this: Rc<T>, other: Rc<T>) -> bool
    where T: std::fmt::Display
{
    let this_ptr: *const T = &*this;
    let other_ptr: *const T = &*other;
    debugPrint!(true, 2, "RC ptr eq {} {:p} {} {:p}", this, this_ptr, other, other_ptr);
    this_ptr == other_ptr
}

fn cast<F>(desc: &str, runtime: &mut Runtime, mutator: F)
    where F: Fn(&Variable) -> Variable
{
    let popped = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
    debugPrint!(true, 2, "{} {}", desc, popped);
    push_on_stack(&mut runtime.current_frame.operand_stack, mutator(&popped));
}

fn ifacmp(desc: &str, runtime: &mut Runtime, buf: &mut Cursor<&Vec<u8>>, should_match: bool) -> Result<(), RunnerError>
{
    let current_position = buf.position() - 1;
    let branch_offset = try!(buf.read_u16::<BigEndian>()) as i16;
    let popped2 = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap().to_ref();
    let popped1 = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap().to_ref();
    debugPrint!(true, 2, "{} {} {} {}", desc, popped1.is_some(), popped2.is_some(), branch_offset);
    let matching = popped1.is_some() == popped2.is_some() && (popped1.is_none() || rc_ptr_eq(popped1.unwrap(), popped2.unwrap()));
    if should_match == matching {
        let new_position = (current_position as i64 + branch_offset as i64) as u64;
        debugPrint!(true, 2, "BRANCHED from {} to {}", current_position, new_position);
        buf.set_position(new_position);
    }
    return Ok(());
}

fn ldc(runtime: &mut Runtime, index: usize) -> Result<(), RunnerError> {
    let maybe_cp_entry = runtime.current_frame.constant_pool.get(&(index as u16)).map(|x| x.clone());
    if maybe_cp_entry.is_none() {
        debugPrint!(true, 1, "LDC failed at index {}", index);
        return Err(RunnerError::ClassInvalid("Error"));
    } else {
        match maybe_cp_entry.as_ref().unwrap() {
            &ConstantPoolItem::CONSTANT_String { index } => {
                let str = try!(get_cp_str(&runtime.current_frame.constant_pool, index));
                debugPrint!(true, 2, "LDC string {}", str);
                let var = try!(make_string(runtime, str.as_str()));
                push_on_stack(&mut runtime.current_frame.operand_stack, var);
            }
            &ConstantPoolItem::CONSTANT_Class { index } => {
                let descriptor = try!(get_cp_str(&runtime.current_frame.constant_pool, index));
                debugPrint!(true, 2, "LDC class {}", descriptor);
                let var = try!(make_class(runtime, descriptor.as_str()));
                push_on_stack(&mut runtime.current_frame.operand_stack, var);
            }
            &ConstantPoolItem::CONSTANT_Integer { value } => {
                debugPrint!(true, 2, "LDC int {}", value as i32);
                push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Int(value as i32));
            }
            &ConstantPoolItem::CONSTANT_Float { value } => {
                debugPrint!(true, 2, "LDC float {}", value as f32);
                push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Float(value as f32));
            }
            _ => return Err(RunnerError::ClassInvalid2(format!("Unknown constant {:?}", maybe_cp_entry.as_ref().unwrap())))
        }
    }
    return Ok(());
}

fn do_run_method(name: &str, runtime: &mut Runtime, code: &Code, pc: u16) -> Result<(), RunnerError> {
    if pc as usize > code.code.len() {
        return Err(RunnerError::InvalidPc);
    }
    let mut buf = Cursor::new(&code.code);

    loop {
        let current_position = buf.position();
        let op_code = try!(buf.read_u8());
        debugPrint!(true, 3, "{} {} Op code {}", name, runtime.count, op_code);
        runtime.count+=1;
        match op_code {
            1 => {
                debugPrint!(true, 2, "ACONST_NULL");
                // Bit weird, use a random class as the type. Probably need a special case for untyped null?
                push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Reference(runtime.classes.values().nth(0).unwrap().clone(), None));
            }
            2...8 => {
                let val = (op_code as i32) - 3;
                debugPrint!(true, 2, "ICONST {}", val);
                push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Int(val));
            }
            9...10 => {
                let val = (op_code as i64) - 9;
                debugPrint!(true, 2, "LCONST {}", val);
                push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Long(val));
            }
            11...13 => {
                let val = (op_code - 11) as f32;
                debugPrint!(true, 2, "FCONST {}", val);
                push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Float(val));
            }
            16 => {
                let byte = try!(buf.read_u8()) as i32;
                debugPrint!(true, 2, "BIPUSH {}", byte);
                push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Int(byte));
            }
            17 => {
                let short = try!(buf.read_u16::<BigEndian>()) as i32;
                debugPrint!(true, 2, "SIPUSH {}", short);
                push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Int(short));
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
                    debugPrint!(true, 1, "LDC2W failed at index {}", index);
                    return Err(RunnerError::ClassInvalid("Error"));
                } else {
                    match maybe_cp_entry.as_ref().unwrap() {
                        &ConstantPoolItem::CONSTANT_Long { value } => {
                            debugPrint!(true, 2, "LDC2W long {}", value);
                            push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Long(value as i64));
                        }
                        &ConstantPoolItem::CONSTANT_Double { value } => {
                            debugPrint!(true, 2, "LDC2W double {}", value);
                            push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Double(value));
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
                debugPrint!(true, 2, "POP {}", popped);
            }
            88 => {
                let popped = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
                if popped.is_type_1() {
                    let popped2 = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
                    debugPrint!(true, 2, "POP2 {} {}", popped, popped2);
                } else {
                    debugPrint!(true, 2, "POP2 {}", popped);
                }
            }
            89 => {
                let stack_len = runtime.current_frame.operand_stack.len();
                let peek = runtime.current_frame.operand_stack[stack_len - 1].clone();
                debugPrint!(true, 2, "DUP {}", peek);
                push_on_stack(&mut runtime.current_frame.operand_stack, peek);
            }
            90 => {
                let stack_len = runtime.current_frame.operand_stack.len();
                let peek = runtime.current_frame.operand_stack[stack_len - 1].clone();
                debugPrint!(true, 2, "DUP_X1 {}", peek);
                runtime.current_frame.operand_stack.insert(stack_len - 2, peek);
            }
            91 => {
                let stack_len = runtime.current_frame.operand_stack.len();
                let peek = runtime.current_frame.operand_stack[stack_len - 1].clone();
                debugPrint!(true, 2, "DUP_X2 {}", peek);
                runtime.current_frame.operand_stack.insert(stack_len - 3, peek);
            }
            92 => {
                let stack_len = runtime.current_frame.operand_stack.len();
                let peek1 = runtime.current_frame.operand_stack[stack_len - 1].clone();
                if peek1.is_type_1() {
                    let peek2 = runtime.current_frame.operand_stack[stack_len - 2].clone();
                    debugPrint!(true, 2, "DUP2 {} {}", peek1, peek2);
                    push_on_stack(&mut runtime.current_frame.operand_stack, peek2);
                    push_on_stack(&mut runtime.current_frame.operand_stack, peek1);
                } else {
                    debugPrint!(true, 2, "DUP2 {}", peek1);
                    push_on_stack(&mut runtime.current_frame.operand_stack, peek1);
                }
            }
            96 => maths_instr("IADD", runtime, Variable::Int, Variable::to_int, add),
            97 => maths_instr("LADD", runtime, Variable::Long, Variable::to_long, add),
            98 => maths_instr("FADD", runtime, Variable::Float, Variable::to_float, add),
            99 => maths_instr("DADD", runtime, Variable::Double, Variable::to_double, add),
            100 => maths_instr("ISUB", runtime, Variable::Int, Variable::to_int, sub),
            101 => maths_instr("LSUB", runtime, Variable::Long, Variable::to_long, sub),
            102 => maths_instr("FSUB", runtime, Variable::Float, Variable::to_float, sub),
            103 => maths_instr("DSUB", runtime, Variable::Double, Variable::to_double, sub),
            104 => maths_instr("IMUL", runtime, Variable::Int, Variable::to_int, mul),
            105 => maths_instr("LMUL", runtime, Variable::Long, Variable::to_long, mul),
            106 => maths_instr("FMUL", runtime, Variable::Float, Variable::to_float, mul),
            107 => maths_instr("DMUL", runtime, Variable::Double, Variable::to_double, mul),
            108 => maths_instr("IDIV", runtime, Variable::Int, Variable::to_int, div),
            109 => maths_instr("LDIV", runtime, Variable::Long, Variable::to_long, div),
            110 => maths_instr("FDIV", runtime, Variable::Float, Variable::to_float, div),
            111 => maths_instr("DDIV", runtime, Variable::Double, Variable::to_double, div),
            112 => maths_instr("IREM", runtime, Variable::Int, Variable::to_int, rem),
            113 => maths_instr("LREM", runtime, Variable::Long, Variable::to_long, rem),
            114 => maths_instr("FREM", runtime, Variable::Float, Variable::to_float, rem),
            115 => maths_instr("DREM", runtime, Variable::Double, Variable::to_double, rem),
            116 => single_pop_instr("INEG", runtime, Variable::Int, Variable::to_int, |x| 0 - x),
            117 => single_pop_instr("LNEG", runtime, Variable::Long, Variable::to_long, |x| 0 - x),
            118 => single_pop_instr("FNEG", runtime, Variable::Float, Variable::to_float, |x| 0.0 - x),
            119 => single_pop_instr("DNEG", runtime, Variable::Double, Variable::to_double, |x| 0.0 - x),
            120 => maths_instr("ISHL", runtime, Variable::Int, Variable::to_int, |x,y| y << x),
            121 => maths_instr_2("LSHL", runtime, Variable::Long, Variable::to_int, Variable::to_long, |x,y| (y << x) as i64),
            122 => maths_instr("ISHR", runtime, Variable::Int, Variable::to_int, |x,y| y >> x),
            123 => maths_instr_2("LSHR", runtime, Variable::Long, Variable::to_int, Variable::to_long, |x,y| (y >> x) as i64),
            124 => maths_instr("IUSHR", runtime, Variable::Int, Variable::to_int, |x,y| ((y as u32)>>x) as i32),
            125 => maths_instr_2("LUSHR", runtime, Variable::Long, Variable::to_int, Variable::to_long, |x,y| ((y as u64)>>x) as i64),
            126 => maths_instr("IAND", runtime, Variable::Int, Variable::to_int, and),
            127 => maths_instr("LAND", runtime, Variable::Long, Variable::to_long, and),
            128 => maths_instr("IOR", runtime, Variable::Int, Variable::to_int, or),
            129 => maths_instr("LOR", runtime, Variable::Long, Variable::to_long, or),
            130 => maths_instr("IXOR", runtime, Variable::Int, Variable::to_int, xor),
            131 => maths_instr("LXOR", runtime, Variable::Long, Variable::to_long, xor),
            132 => {
                let index = try!(buf.read_u8());
                let constt = try!(buf.read_u8()) as i8;
                debugPrint!(true, 2, "IINC {} {}", index, constt);
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
                debugPrint!(true, 2, "LCMP {} {}", pop1, pop2);
                let ret;
                if pop1 > pop2 {
                    ret = 1;
                } else if pop1 == pop2 {
                    ret = 0;
                } else {
                    ret = -1;
                }
                push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Int(ret));
            }
            149 => try!(fcmp("FCMPG", runtime, true)),
            150 => try!(fcmp("FCMPL", runtime, false)),
            153 => try!(ifcmp("IFEQ", runtime, &mut buf, |x| x == 0)),
            154 => try!(ifcmp("IFNE", runtime, &mut buf, |x| x != 0)),
            155 => try!(ifcmp("IFLT", runtime, &mut buf, |x| x < 0)),
            156 => try!(ifcmp("IFGE", runtime, &mut buf, |x| x >= 0)),
            157 => try!(ifcmp("IFGT", runtime, &mut buf, |x| x > 0)),
            158 => try!(ifcmp("IFLE", runtime, &mut buf, |x| x <= 0)),
            159 => try!(icmp("IF_ICMPEQ", runtime, &mut buf, |x,y| x == y)),
            160 => try!(icmp("IF_ICMPNE", runtime, &mut buf, |x,y| x != y)),
            161 => try!(icmp("IF_ICMPLT", runtime, &mut buf, |x,y| x < y)),
            162 => try!(icmp("IF_ICMPGE", runtime, &mut buf, |x,y| x >= y)),
            163 => try!(icmp("IF_ICMPGT", runtime, &mut buf, |x,y| x > y)),
            164 => try!(icmp("IF_ICMPLE", runtime, &mut buf, |x,y| x <= y)),
            165 => try!(ifacmp("IF_ACMPEQ", runtime, &mut buf, true)),
            166 => try!(ifacmp("IF_ACMPNEQ", runtime, &mut buf, false)),
            167 => {
                let branch_offset = try!(buf.read_u16::<BigEndian>()) as i16;
                let new_pos = (current_position as i64 + branch_offset as i64) as u64;
                debugPrint!(true, 2, "BRANCH from {} to {}", current_position, new_pos);
                buf.set_position(new_pos);
            }
            172 => { return vreturn("IRETURN", runtime, Variable::can_convert_to_int); }
            173 => { return vreturn("LRETURN", runtime, Variable::to_long); }
            174 => { return vreturn("FRETURN", runtime, Variable::to_float); }
            175 => { return vreturn("DRETURN", runtime, Variable::to_double); }
            176 => { return vreturn("ARETURN", runtime, Variable::is_ref_or_array); }
            177 => { // return
                debugPrint!(true, 1, "RETURN");
                runtime.current_frame = runtime.previous_frames.pop().unwrap();
                return Ok(());
            }
            178 => { // getstatic
                let index = try!(buf.read_u16::<BigEndian>());
                let (class_name, field_name, typ) = try!(get_cp_field(&runtime.current_frame.constant_pool, index));
                debugPrint!(true, 2, "GETSTATIC {} {} {}", class_name, field_name, typ);
                let mut class_result = try!(load_class(runtime, class_name.as_str()));
                loop {
                    {
                        let statics = class_result.statics.borrow();
                        let maybe_static_variable = statics.get(&*field_name);
                        if maybe_static_variable.is_some() {
                            push_on_stack(&mut runtime.current_frame.operand_stack, maybe_static_variable.unwrap().clone());
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
                debugPrint!(true, 2, "PUTSTATIC {} {} {} {}", class_name, field_name, typ, value);
                try!(put_static(runtime, class_name.as_str(), field_name.as_str(), value));
            }
            180 => {
                let field_index = try!(buf.read_u16::<BigEndian>());
                let (class_name, field_name, typ) = try!(get_cp_field(&runtime.current_frame.constant_pool, field_index));
                let var = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
                let obj = try!(try!(get_obj_instance_from_variable(&var)).ok_or(RunnerError::NullPointerException));
                debugPrint!(true, 2, "GETFIELD class:'{}' field:'{}' type:'{}' object:'{}'", class_name, field_name, typ, obj);
                let f = try!(get_field(&obj, class_name.as_str(), field_name.as_str()));
                push_on_stack(&mut runtime.current_frame.operand_stack, f);
            }
            181 => {
                let field_index = try!(buf.read_u16::<BigEndian>());
                let (class_name, field_name, typ) = try!(get_cp_field(&runtime.current_frame.constant_pool, field_index));
                let value = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
                let var = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
                let obj = try!(try!(get_obj_instance_from_variable(&var)).ok_or(RunnerError::NullPointerException));
                debugPrint!(true, 2, "PUTFIELD {} {} {} {} {}", class_name, field_name, typ, obj, value);
                try!(put_field(obj, class_name.as_str(), field_name.as_str(), value));
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
                debugPrint!(true, 2, "NEW {}", class_name);
                let var = try!(construct_object(runtime, class_name.as_str()));
                push_on_stack(&mut runtime.current_frame.operand_stack, var);
            }
            188 => {
                let atype = try!(buf.read_u8());
                let count = try!(pop_from_stack(&mut runtime.current_frame.operand_stack).ok_or(RunnerError::ClassInvalid("NEWARRAY POP fail"))).to_int();
                debugPrint!(true, 2, "NEWARRAY {} {}", atype, count);
                let mut v : Vec<Variable> = Vec::new();
                for _c in 0..count {
                    v.push(
                        match atype {
                            4 => Variable::Boolean(false),
                            5 => Variable::Char('\0'),
                            6 => Variable::Float(0.0),
                            7 => Variable::Double(0.0),
                            8 => Variable::Byte(0),
                            9 => Variable::Short(0),
                            10 => Variable::Int(0),
                            11 => Variable::Long(0),
                            _ => return Err(RunnerError::ClassInvalid2(format!("New array type {} unknown", atype)))
                        });
                }
                push_on_stack(&mut runtime.current_frame.operand_stack, Variable::ArrayReference(Rc::new(v[0].clone()), Some(Rc::new(RefCell::new(v)))));
            }
            189 => {
                let index = try!(buf.read_u16::<BigEndian>());
                let class_name = try!(get_cp_class(&runtime.current_frame.constant_pool, index));
                try!(load_class(runtime, class_name.as_str()));
                let class = runtime.classes.get(&*class_name).unwrap();
                let count = try!(pop_from_stack(&mut runtime.current_frame.operand_stack).ok_or(RunnerError::ClassInvalid("ANEWARRAY count fail"))).to_int();
                debugPrint!(true, 2, "ANEWARRAY {} {}", class_name, count);
                let mut v : Vec<Variable> = Vec::new();
                for _c in 0..count {
                    v.push(Variable::Reference(class.clone(), None));
                }
                push_on_stack(&mut runtime.current_frame.operand_stack, Variable::ArrayReference(Rc::new(Variable::Reference(class.clone(), None)), Some(Rc::new(RefCell::new(v)))));
            }
            190 => {
                let var = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
                let (typee, array) = var.to_arrayref();
                if array.is_none() {
                    return Err(RunnerError::NullPointerException);
                }
                let len = array.as_ref().unwrap().borrow().len();
                debugPrint!(true, 2, "ARRAYLEN {} {} {}", var, typee, len);
                push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Int(len as i32));
            }
            192 => {
                let var = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
                let index = try!(buf.read_u16::<BigEndian>());

                debugPrint!(true, 2, "CHECKCAST {} {}", var, index);

                let maybe_cp_entry = runtime.current_frame.constant_pool.get(&index);
                if maybe_cp_entry.is_none() {
                    debugPrint!(true, 1, "Missing CP class {}", index);
                    return Err(RunnerError::ClassInvalid("Error"));
                } else {
                    // TODO: CHECKCAST (noop)
                    push_on_stack(&mut runtime.current_frame.operand_stack, var);
                }
            }
            193 => {
                let var = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
                let index = try!(buf.read_u16::<BigEndian>());
                let class_name = try!(get_cp_class(&runtime.current_frame.constant_pool, index));

                debugPrint!(true, 2, "INSTANCEOF {} {}", var, class_name);

                let var_ref = var.to_ref();
                let mut matches = false;
                if var_ref.is_some() {
                    let mut obj = get_most_sub_class(var_ref.unwrap());

                    // Search down to find if instance of
                    while {matches = obj.type_ref.name == *class_name; obj.super_class.borrow().is_some()} {
                        if matches {
                            break;
                        }
                        let new_obj = obj.super_class.borrow().as_ref().unwrap().clone();
                        obj = new_obj;
                    }
                }
                push_on_stack(&mut runtime.current_frame.operand_stack, Variable::Int(if matches {1} else {0}));
            }
            194 => {
                let var = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
                debugPrint!(true, 2, "MONITORENTER {}", var);
                let _obj = try!(try!(get_obj_instance_from_variable(&var)).ok_or(RunnerError::NullPointerException));
                // TODO: Implement monitor
                debugPrint!(true, 1, "WARNING: MonitorEnter not implemented");
            },
            195 => {
                let var = pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap();
                debugPrint!(true, 2, "MONITOREXIT {}", var);
                let _obj = try!(try!(get_obj_instance_from_variable(&var)).ok_or(RunnerError::NullPointerException));
                // TODO: Implement monitor
                debugPrint!(true, 1, "WARNING: MonitorExit not implemented");
            },
            198 => try!(branch_if("IFNULL", runtime, &mut buf, current_position, |x| x.is_null())),
            199 => try!(branch_if("IFNONNULL", runtime, &mut buf, current_position, |x| !x.is_null())),
            _ => return Err(RunnerError::UnknownOpCode(op_code))
        }
    }
}

fn find_class(base_name: &str, class_paths: &Vec<String>) -> Result<ClassResult, RunnerError> {
    let debug = false;
    let mut name = String::from(base_name);
    name = name.replace('.', "/");
    debugPrint!(debug, 3, "Finding class {}", name);
    for class_path in class_paths.iter() {
        let mut direct_path = PathBuf::from(class_path);
        for sub in name.split('/') {
            direct_path.push(sub)
        }
        direct_path.set_extension("class");
        debugPrint!(debug, 3, "Trying path {}", direct_path.display());
        let direct_classname = get_classname(direct_path.as_path());
        if direct_classname.is_ok() && *direct_classname.as_ref().unwrap() == name {
            debugPrint!(debug, 3, "Name matched for {}", name);
            let maybe_read = read(Path::new(&direct_path));
            if maybe_read.is_ok() {
                return Ok(maybe_read.unwrap());
            }
        }

        if false {
            debugPrint!(debug, 3, "Finding class {} direct load failed ({}), searching {}",
                name, match &direct_classname {
                    &Ok(ref x) => x.clone(),
                    &Err(ref y) => format!("{:?}", y),
                }, class_path);

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
                .filter(|x| {
                    let classname = get_classname(&x);
                    return classname.is_ok() && classname.unwrap() == name;
                })
                .nth(0);

            if class_match.is_none() {
                debugPrint!(debug, 2, "Could not find {} on class path {}", name, class_path);
                continue;
            }

            let maybe_read = read(&class_match.unwrap());
            if maybe_read.is_err() {
                debugPrint!(true, 1, "Error reading class {} on class path {}", name, class_path);
                continue;
            }

            return Ok(maybe_read.unwrap());
        } else {
            debugPrint!(debug, 2, "Could not find {} on class path {} (Error {:?})", name, class_path, direct_classname);
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
    debugPrint!(true, 2, "Finding class {} not already loaded", name);
    let class_result = try!(find_class(name, &runtime.class_paths));
    let class_obj = try!(bootstrap_class_and_dependencies(runtime, name, &class_result));

    return Ok(class_obj);
}

fn bootstrap_class_and_dependencies(runtime: &mut Runtime, name: &str, class_result: &ClassResult) -> Result<Rc<Class>, RunnerError>  {
    let debug = true;
    let mut unresolved_classes : Vec<String> = Vec::new();
    let mut classes_to_process : Vec<Rc<Class>> = Vec::new();

    let new_class = Rc::new(Class::new(&String::from(name), class_result));
    runtime.classes.insert(String::from(name), new_class.clone());
    classes_to_process.push(new_class);
    debugPrint!(debug, 1, "Bootstrapping {}", name);
    try!(find_unresolved_class_dependencies(runtime, &mut unresolved_classes, class_result));

    while unresolved_classes.len() > 0 {
        let class_to_resolve = unresolved_classes.pop().unwrap().clone();
        if !runtime.classes.contains_key(&class_to_resolve) {
            debugPrint!(debug, 2, "Finding unresolved dependencies in class {}", class_to_resolve);
            let class_result_to_resolve = try!(find_class(&class_to_resolve, &runtime.class_paths));
            let new_class = Rc::new(Class::new(&class_to_resolve, &class_result_to_resolve));
            runtime.classes.insert(class_to_resolve, new_class.clone());
            classes_to_process.push(new_class);
            try!(find_unresolved_class_dependencies(runtime, &mut unresolved_classes, &class_result_to_resolve));
        }
    }

    for class in &classes_to_process {
        try!(initialise_class_stage_1(runtime, class));
    }

    let my_class = runtime.classes.get(&String::from(name)).unwrap().clone();
    try!(initialise_class_stage_2(runtime, &my_class));
    debugPrint!(debug, 1, "Bootstrap totally complete on {}", name);
    return Ok(my_class);
}

fn find_unresolved_class_dependencies(runtime: &mut Runtime, unresolved_classes: &mut Vec<String>, class_result: &ClassResult) -> Result<(), RunnerError> {
    let debug = false;
    for field in &class_result.fields {
        let name_string = try!(get_cp_str(&class_result.constant_pool, field.name_index));
        let descriptor_string = try!(get_cp_str(&class_result.constant_pool, field.descriptor_index));

        debugPrint!(debug, 3, "Checking field {} {}", name_string, descriptor_string);

        let variable = try!(parse_single_type_string(runtime, descriptor_string.as_str(), false));
        match variable {
            Variable::UnresolvedReference(ref type_string) => {
                debugPrint!(debug, 3, "Class {} is unresolved", type_string);
                unresolved_classes.push(type_string.clone());
            },
            _ => {}
        }
    }

    if class_result.super_class_index > 0 {
        let class_name = try!(get_cp_class(&class_result.constant_pool, class_result.super_class_index));
        if !runtime.classes.contains_key(&*class_name) {
            unresolved_classes.push((*class_name).clone());
        }
    }
    if !runtime.classes.contains_key(&String::from("java/lang/Object")) {
        unresolved_classes.push(String::from("java/lang/Object"));
    }
    return Ok(());
}

fn initialise_class_stage_1(runtime: &mut Runtime, class: &Rc<Class>) -> Result<(), RunnerError> {
    let debug = false;
    if *class.initialising.borrow() || *class.initialised.borrow() {
        return Ok(());
    }
    debugPrint!(debug, 2, "Initialising class stage 1 {}", class.name);

    for field in &class.cr.fields {
        if field.access_flags & ACC_STATIC == 0 {
            continue;
        }

        let name_string = try!(get_cp_str(&class.cr.constant_pool, field.name_index));
        let descriptor_string = try!(get_cp_str(&class.cr.constant_pool, field.descriptor_index));

        debugPrint!(debug, 3, "Constructing class static member {} {}", name_string, descriptor_string);

        let var = try!(initialise_variable(runtime, descriptor_string.as_str()));

        class.statics.borrow_mut().insert((*name_string).clone(), var);
    }
    if class.cr.super_class_index > 0 {
        let super_class_name = try!(get_cp_class(&class.cr.constant_pool, class.cr.super_class_index));
        debugPrint!(debug, 3, "Class {} has superclass {}", class.name, super_class_name);
        *class.super_class.borrow_mut() = Some(try!(runtime.classes.get(&*super_class_name).ok_or(RunnerError::ClassInvalid("Error"))).clone());
    } else {
        if class.name != "java/lang/Object" {
            debugPrint!(debug, 3, "Class {} has superclass {}", class.name, "java/lang/Object");
            *class.super_class.borrow_mut() = Some(try!(runtime.classes.get(&String::from("Java/lang/Object")).ok_or(RunnerError::ClassInvalid("Error"))).clone());
        }
    }
    return Ok(());
}

fn initialise_class_stage_2(runtime: &mut Runtime, class: &Rc<Class>) -> Result<(), RunnerError> {
    if *class.initialising.borrow() || *class.initialised.borrow() {
        return Ok(());
    }
    debugPrint!(true, 2, "Initialising class stage 2 {}", class.name);

    *class.initialising.borrow_mut() = true;
    try!(invoke_manual(runtime, class.clone(), Vec::new(), "<clinit>", "()V", true));
    *class.initialised.borrow_mut() = true;

    return Ok(());
}

fn generate_variable_descriptor(var: &Variable) -> String {
    let mut ret = String::new();
    match var {
        &Variable::Byte(_v) => {ret.push('B');},
        &Variable::Char(_v) => {ret.push('C');},
        &Variable::Double(_v) => {ret.push('D');},
        &Variable::Float(_v) => {ret.push('F');},
        &Variable::Int(_v) => {ret.push('I');},
        &Variable::Long(_v) => {ret.push('J');},
        &Variable::Short(_v) => {ret.push('S');},
        &Variable::Boolean(_v) => {ret.push('Z');},
        &Variable::Reference(ref class, ref _obj) => {
            ret.push('L');
            ret.push_str(class.name.as_str());
            ret.push(';');
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

fn generate_method_descriptor(args: &Vec<Variable>, return_type: Option<&Variable>, is_static: bool) -> String {
    let mut ret = String::new();
    ret.push('(');
    for arg in args.iter().skip(if is_static {0} else {1}) {
        ret.push_str(generate_variable_descriptor(arg).as_str());
    }
    ret.push(')');
    if return_type.is_some() {
        ret.push_str(generate_variable_descriptor(return_type.unwrap()).as_str());
    } else {
        ret.push('V');
    }
    return ret;
}

fn parse_single_type_string(runtime: &mut Runtime, string: &str, resolve: bool) -> Result<Variable, RunnerError> {
    let mut iter = string.chars();

    let mut maybe_type_specifier = iter.next();

    if maybe_type_specifier.is_none() {
        debugPrint!(true, 2, "Type specifier blank");
        return Err(RunnerError::ClassInvalid("Error"));
    }

    let mut array_depth = 0;
    while maybe_type_specifier.unwrap_or(' ') == '[' {
        array_depth = array_depth + 1;
        maybe_type_specifier = iter.next();
    }

    if maybe_type_specifier.is_none() {
        debugPrint!(true, 2, "Type specifier invalid {}", string);
        return Err(RunnerError::ClassInvalid("Error"));
    }

    let mut variable;
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
                variable = Variable::Reference(try!(load_class(runtime, type_string.as_str())), None);
            } else {
                if runtime.classes.contains_key(type_string.as_str()) {
                    let class = runtime.classes.get(type_string.as_str()).unwrap().clone();
                    variable = Variable::Reference(class.clone(), None);
                } else {
                    variable = Variable::UnresolvedReference(type_string.clone());
                }
            }
        }
    }

    if array_depth > 0 {
        if array_depth > 1 {
            panic!("Unsupported multidimensional array");
        } else {
            variable = Variable::ArrayReference(Rc::new(variable), None);
        }
    }

    return Ok(variable);
}

fn parse_function_type_string(runtime: &mut Runtime, string: &str) -> Result<(Vec<Variable>, Option<Variable>), RunnerError> {
    let debug = false;
    let mut iter = string.chars().peekable();

    if iter.next().unwrap_or(' ') != '(' {
        debugPrint!(true, 2, "Type {} invalid", string);
        return Err(RunnerError::ClassInvalid("Error"));
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
        debugPrint!(debug, 3, "Found parameter {}", type_string);
        let param = try!(parse_single_type_string(runtime, type_string.as_str(), false));
        if !param.is_type_1() {
            parameters.push(param.clone());
        }
        parameters.push(param);
        debugPrint!(debug, 3, "Parameters now {:?}", parameters);
    }

    let return_type_string : String = iter.collect();
    debugPrint!(debug, 3, "Return type {}", return_type_string);
    if return_type_string == "V" {
        return Ok((parameters, None));
    } else {
        return Ok((parameters, Some(try!(parse_single_type_string(runtime, return_type_string.as_str(), false)))));
    }
}

pub fn run(class_paths: &Vec<String>, class: &ClassResult) -> Result<(), RunnerError> {
    println!("Running");
    let mut runtime = Runtime::new(class_paths.clone(), class.constant_pool.clone());

    try!(bootstrap_class_and_dependencies(&mut runtime, String::new().as_str(), class));

    let main_code = try!(get_class_method_code(class, &"main", &"([Ljava/lang/String;)V"));

    try!(do_run_method("main", &mut runtime, &main_code, 0));

    return Ok(());
}

pub fn run_method(class_paths: &Vec<String>, class_result: &ClassResult, method: &str, arguments: &Vec<Variable>, return_type: Option<&Variable>) -> Result<Variable, RunnerError> {
    println!("Running method {} with {} arguments", method, arguments.len());
    let mut runtime = Runtime::new(class_paths.clone(), class_result.constant_pool.clone());

    let name = try!(class_result.name());
    let class = try!(bootstrap_class_and_dependencies(&mut runtime, name.as_str(), class_result));

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

    let method_descriptor = generate_method_descriptor(&arguments, return_type, true);
    debugPrint!(true, 1, "Finding method {} with descriptor {}", method, method_descriptor);
    let code = try!(get_class_method_code(class_result, method, method_descriptor.as_str()));

    println!("Running method");
    try!(do_run_method(method, &mut runtime, &code, 0));

    return Ok(pop_from_stack(&mut runtime.current_frame.operand_stack).unwrap().clone());
}