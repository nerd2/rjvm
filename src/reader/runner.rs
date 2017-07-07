extern crate byteorder;
#[macro_use]
use reader::class::*;
use std;
use std::fmt;
use std::io;
use std::io::Cursor;
use std::ops::Add;
use std::ops::Sub;
use std::ops::Mul;
use std::ops::Div;
use std::ops::Rem;
use std::ops::Shl;
use std::ops::Shr;
use std::ops::BitAnd;
use std::ops::BitOr;
use std::ops::BitXor;
use std::collections::{HashMap, HashSet};
use std::cell::RefCell;
use std::rc::Rc;
use std::rc::Weak;
use std::path::{Path, PathBuf};
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
    initialised: RefCell<bool>,
    statics: RefCell<HashMap<String, Variable>>,
    super_class: RefCell<Option<Rc<Class>>>
}
impl Class {
  pub fn new(name: &String, cr: &ClassResult) -> Class {
      return Class { name: name.clone(), initialised: RefCell::new(false), cr: cr.clone(), statics: RefCell::new(HashMap::new()), super_class: RefCell::new(None)};
  }
}

#[derive(Clone, Debug)]
pub struct Object {
    typeRef: Rc<Class>,
    members: RefCell<HashMap<String, Variable>>,
    super_class: RefCell<Option<Rc<Object>>>,
    sub_class: RefCell<Option<Weak<Object>>>
}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Object type:{}", self.typeRef.name)
    }
}
impl PartialEq for Object { // Have to implement PartialEq because not derrivable for Weaks in general. We can assume the weak ref is constant.
    fn eq(&self, other: &Self) -> bool {
        let self_sub_class = self.sub_class.borrow();
        let other_sub_class = other.sub_class.borrow();

        return self.typeRef == other.typeRef &&
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
    pub fn to_int(&self) -> i32 {
        match self {
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
    pub fn to_ref(&self) -> Option<Rc<Object>> {
        match self {
            &Variable::Reference(ref class, ref obj) => {
                return obj.clone();
            },
            _ => {
                panic!("Couldn't convert to reference");
            }
        }
    }
    pub fn is_ref_or_array(&self) -> bool {
        match self {
            &Variable::Reference(ref class, ref obj) => {
                return true;
            },
            &Variable::ArrayReference(ref typee, ref array) => {
                return true;
            },
            _ => {
                panic!("Couldn't convert to reference or array");
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
    count: i64
}

fn last_mut(v : &mut Vec<Frame>) -> &mut Frame {
    let len = v.len();
    return &mut v[len-1];
}

impl From<io::Error> for RunnerError {
    fn from(err: io::Error) -> RunnerError {
        RunnerError::IoError
    }
}

fn get_cp_str(constant_pool: &HashMap<u16, ConstantPoolItem>, index:u16) -> Result<&str, RunnerError> {
    let maybe_cp_entry = constant_pool.get(&index);
    if maybe_cp_entry.is_none() {
        debugPrint!(true, 1, "Missing CP string {}", index);
        return Err(RunnerError::ClassInvalid2(format!("Missing CP string {}", index)));
    } else {
        match *maybe_cp_entry.unwrap() {
            ConstantPoolItem::CONSTANT_Utf8(ref s) => {
                return Ok(&s);
            }
            _ => {
                debugPrint!(true, 1, "CP item at index {} is not utf8", index);
                return Err(RunnerError::ClassInvalid("Error"));
            }
        }
    }
}

fn get_cp_class(constant_pool: &HashMap<u16, ConstantPoolItem>, index: u16) -> Result<&str, RunnerError> {
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

fn get_cp_name_and_type(constant_pool: &HashMap<u16, ConstantPoolItem>, index: u16) -> Result<(&str, &str), RunnerError> {
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

fn get_cp_field(constant_pool: &HashMap<u16, ConstantPoolItem>, index: u16) -> Result<(&str, &str, &str), RunnerError> {
    debugPrint!(false, 5, "{}", index);
    let maybe_cp_entry = constant_pool.get(&index);
    if maybe_cp_entry.is_none() {
        debugPrint!(true, 1, "Missing CP field {}", index);
        return Err(RunnerError::ClassInvalid("Error"));
    } else {
        match *maybe_cp_entry.unwrap() {
            ConstantPoolItem::CONSTANT_Fieldref{class_index, name_and_type_index} => {
                let class_str = try!(get_cp_class(constant_pool, class_index));
                let (name_str, type_str) = try!(get_cp_name_and_type(constant_pool, name_and_type_index));
                return Ok((class_str, name_str, type_str));
            }
            _ => {
                println!("Index {} is not a field", index);
                return Err(RunnerError::ClassInvalid("Error"));
            }
        }
    }
}

fn get_cp_method(constant_pool: &HashMap<u16, ConstantPoolItem>, index: u16) -> Result<(&str, &str, &str), RunnerError> {
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

fn initialise_variable(classes: &HashMap<String, Rc<Class>>, descriptor_string: &str) -> Result<Variable, RunnerError> {
    let mut variable = try!(parse_single_type_string(classes, descriptor_string));
    return Ok(variable);
}

fn construct_object(classes: &mut HashMap<String, Rc<Class>>, name: &str, class_paths: &Vec<String>) -> Result<Variable, RunnerError> {
    let debug = false;
    debugPrint!(true, 3, "Constructing object {}", name);
    try!(load_class(classes, name, class_paths));

    let original_class = try!(classes.get(name).ok_or(RunnerError::ClassInvalid("Error")));
    let mut original_obj : Option<Rc<Object>> = None;
    let mut class = original_class.clone();
    let mut sub_class : Option<Weak<Object>> = None;

    while true {
        debugPrint!(debug, 3, "Constructing object of type {} with subclass {}", class.name, sub_class.is_some());
        let mut members: HashMap<String, Variable> = HashMap::new();
        for field in &class.cr.fields {
            if field.access_flags & ACC_STATIC != 0 {
                continue;
            }

            let name_string = try!(get_cp_str(&class.cr.constant_pool, field.name_index));
            let descriptor_string = try!(get_cp_str(&class.cr.constant_pool, field.descriptor_index));

            let var = try!(initialise_variable(classes, descriptor_string));

            members.insert(String::from(name_string), var);
        }

        let obj = Rc::new(Object { typeRef: class.clone(), members: RefCell::new(members), super_class: RefCell::new(None), sub_class: RefCell::new(sub_class.clone()) });
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
    return Err(RunnerError::ClassInvalid("Error"));
}

fn get_class_method_code(class: &ClassResult, target_method_name: &str, target_descriptor: &str) -> Result<Code, RunnerError> {
    let debug = true;
    let class_name = try!(get_cp_class(&class.constant_pool, class.this_class_index));
    let mut method_res: Result<&FieldItem, RunnerError> = Err(RunnerError::ClassInvalid2(format!("Could not find method {} with descriptor {}", target_method_name, target_descriptor)));

    for method in &class.methods {
        let method_name = try!(get_cp_str(&class.constant_pool, method.name_index));
        let descriptor = try!(get_cp_str(&class.constant_pool, method.descriptor_index));
        debugPrint!(debug, 3, "Checking method {} {}", method_name, descriptor);
        if method_name == target_method_name &&
            descriptor == target_descriptor {
            method_res = Ok(method);
            break;
        }
    }

    let method = try!(method_res);
    debugPrint!(debug, 3, "Found method");
    if method.access_flags & ACC_NATIVE != 0 {
        return Err(RunnerError::NativeMethod(format!("Method '{}' in class '{}'", target_method_name, class_name)));
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
        &Variable::Reference(ref class, ref objref) => {
            return Ok(objref.clone());
        },
        _ => {
            return Err(RunnerError::ClassInvalid("Error"));
        }
    }
}

fn construct_char_array(s: &str) -> Variable {
    let mut v : Vec<Variable> = Vec::new();
    for c in s.chars() {
        v.push(Variable::Char(c));
    }
    return Variable::ArrayReference(Rc::new(Variable::Char('\0')), Some(Rc::new(RefCell::new(v))));
}

fn load<F>(desc: &str, index: u8, mut runtime: &mut Runtime, t: F) -> Result<(), RunnerError> { // TODO: Type checking
    let loaded = runtime.current_frame.local_variables[index as usize].clone();
    debugPrint!(true, 2, "{} {} {}", desc, index, loaded);
    runtime.current_frame.operand_stack.push(loaded);
    return Ok(());
}

fn aload<F>(desc: &str, mut runtime: &mut Runtime, t: F) -> Result<(), RunnerError> { // TODO: Type checking
    let index = runtime.current_frame.operand_stack.pop().unwrap().to_int();
    let var = runtime.current_frame.operand_stack.pop().unwrap();
    let (array_type, maybe_array) = var.to_arrayref();
    debugPrint!(true, 2, "{} {} {:?}", desc, index, maybe_array);
    if maybe_array.is_none() {
        return Err(RunnerError::NullPointerException);
    }

    let array = maybe_array.as_ref().unwrap().borrow();
    if array.len() < index as usize {
        return Err(RunnerError::ArrayIndexOutOfBoundsException(array.len(), index as usize));
    }

    runtime.current_frame.operand_stack.push(array[index as usize].clone());
    return Ok(());
}

fn store<F>(desc: &str, index: u8, mut runtime: &mut Runtime, t: F) -> Result<(), RunnerError> { // TODO: Type checking
    let popped = runtime.current_frame.operand_stack.pop().unwrap();
    debugPrint!(true, 2, "{}_{} {}", desc, index, popped);
    let local_len = runtime.current_frame.local_variables.len();
    if local_len > index as usize {
        runtime.current_frame.local_variables[index as usize] = popped;
    } else if local_len == index as usize {
        runtime.current_frame.local_variables.push(popped);
    } else {
        debugPrint!(true, 1, "Asked to store into local variables at index {} when current size is only {}", index, local_len);
        return Err(RunnerError::InvalidPc);
    }
    return Ok(());
}


fn astore<F>(desc: &str, mut runtime: &mut Runtime, t: F) -> Result<(), RunnerError> { // TODO: Type checking
    let value = runtime.current_frame.operand_stack.pop().unwrap();
    let index = runtime.current_frame.operand_stack.pop().unwrap().to_int();
    let var = runtime.current_frame.operand_stack.pop().unwrap();
    let (array_type, maybe_array) = var.to_arrayref();
    debugPrint!(true, 2, "{} {} {:?}", desc, index, maybe_array);
    if maybe_array.is_none() {
        return Err(RunnerError::NullPointerException);
    }

    let mut array = maybe_array.as_ref().unwrap().borrow_mut();
    if array.len() < index as usize {
        return Err(RunnerError::ArrayIndexOutOfBoundsException(array.len(), index as usize));
    }

    array[index as usize] = value;
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

fn maths_instr<F, G, H, K>(desc: &str, mut runtime: &mut Runtime, creator: F, extractor: G, operation: H)
    where
    F: Fn(K) -> Variable,
    G: Fn(&Variable) -> K,
    H: Fn(K, K) -> K
{
    let popped1 = runtime.current_frame.operand_stack.pop().unwrap();
    let popped2 = runtime.current_frame.operand_stack.pop().unwrap();
    debugPrint!(true, 2, "{} {} {}", desc, popped1, popped2);
    runtime.current_frame.operand_stack.push(creator(operation(extractor(&popped1), extractor(&popped2))));
}

fn maths_instr_2<F, G, H, I, J, K, L>(desc: &str, mut runtime: &mut Runtime, creator: F, extractor1: G, extractor2: H, operation: I)
    where
        F: Fn(L) -> Variable,
        G: Fn(&Variable) -> J,
        H: Fn(&Variable) -> K,
        I: Fn(J, K) -> L
{
    let popped1 = runtime.current_frame.operand_stack.pop().unwrap();
    let popped2 = runtime.current_frame.operand_stack.pop().unwrap();
    debugPrint!(true, 2, "{} {} {}", desc, popped1, popped2);
    runtime.current_frame.operand_stack.push(creator(operation(extractor1(&popped1), extractor2(&popped2))));
}

fn single_pop_instr<F, G, H, I, J>(desc: &str, mut runtime: &mut Runtime, creator: F, extractor: G, operation: H)
    where
    F: Fn(J) -> Variable,
    G: Fn(&Variable) -> I,
    H: Fn(I) -> J
{
    let popped = runtime.current_frame.operand_stack.pop().unwrap();
    debugPrint!(true, 2, "{} {}", desc, popped);
    runtime.current_frame.operand_stack.push(creator(operation(extractor(&popped))));
}

fn vreturn<F, K>(desc: &str, mut runtime: &mut Runtime, extractor: F) -> Result<(), RunnerError> where F: Fn(&Variable) -> K {
    let popped = runtime.current_frame.operand_stack.pop().unwrap();
    debugPrint!(true, 2, "{} {}", desc, popped);
    extractor(&popped); // Type check
    runtime.current_frame = runtime.previous_frames.pop().unwrap();
    runtime.current_frame.operand_stack.push(popped);
    return Ok(());
}

// Get the (super)object which contains a field
fn get_obj_field(mut obj: Rc<Object>, field_name: &str) -> Result<Rc<Object>, RunnerError> {
    let class_name = obj.typeRef.name.clone();
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
    while obj.typeRef.name != class_name && obj.super_class.borrow().is_some() {
        let new_obj = obj.super_class.borrow().clone().unwrap();
        obj = new_obj;
        debugPrint!(false, 3, "Class didn't match, checking {} now)", obj.typeRef.name);
    }

    if obj.typeRef.name != class_name {
        debugPrint!(true, 1, "Expected object on stack with class name {} but got {}", class_name, obj.typeRef.name);
        return Err(RunnerError::ClassInvalid2(format!("Couldn't find object on stack with class name {}", class_name)));
    }

    return Ok(obj);
}

fn invoke_manual(mut runtime: &mut Runtime, obj: Rc<Object>, args: Vec<Variable>, method_name: &str, method_descriptor: &str) -> Result<(), RunnerError>{
    debugPrint!(true, 3, "Invoking manually {} {} on {}", method_name, method_descriptor, obj);
    let new_frame = Frame {
        constant_pool: obj.typeRef.cr.constant_pool.clone(),
        operand_stack: Vec::new(),
        local_variables: args.clone()};

    let code = try!(get_class_method_code(&obj.typeRef.cr, method_name, method_descriptor));

    runtime.previous_frames.push(runtime.current_frame.clone());
    runtime.current_frame = new_frame;
    try!(do_run_method(&mut runtime, &code, 0));

    return Ok(());
}

fn invoke(desc: &str, mut runtime: &mut Runtime, index: u16, with_obj: bool, special: bool) -> Result<(), RunnerError> {
    let debug = false;
    let mut code : Option<Code> = None;
    let mut new_frame : Option<Frame> = None;
    let current_op_stack_size = runtime.current_frame.operand_stack.len();

    {
        let (class_name, method_name, descriptor) = try!(get_cp_method(&runtime.current_frame.constant_pool, index));
        let (parameters, return_type) = try!(parse_function_type_string(&runtime.classes, descriptor));
        let extra_parameter = if with_obj {1} else {0};
        let new_local_variables = runtime.current_frame.operand_stack.split_off(current_op_stack_size - parameters.len() - extra_parameter);

        let mut class = try!(load_class(&mut runtime.classes, class_name, &runtime.class_paths));

        if with_obj {
            let mut obj = new_local_variables[0].to_ref().unwrap();

            if special {
                while obj.typeRef.name != class_name {
                    let new_obj = obj.super_class.borrow().as_ref().unwrap().clone();
                    obj = new_obj;
                }
            } else {
                // Virtual: find topmost object
                while obj.sub_class.borrow().is_some() {
                    let new_obj = obj.sub_class.borrow().as_ref().unwrap().upgrade().unwrap();
                    obj = new_obj;
                }
            }

            // Find method
            while { code = get_class_method_code(&obj.typeRef.cr, method_name, descriptor).ok(); code.is_none() } {
                if obj.super_class.borrow().is_none() {
                    return Err(RunnerError::ClassInvalid2(format!("Could not find super class of object that matched method {} {}", method_name, descriptor)))
                }
                let new_obj = obj.super_class.borrow().clone().unwrap();
                obj = new_obj;
            }
            class = obj.typeRef.clone();
        } else {
            code = Some(try!(get_class_method_code(&class.cr, method_name, descriptor)));
        }

        debugPrint!(true, 1, "{} {} {} {}", desc, class_name, method_name, descriptor);
        new_frame = Some(Frame {
            constant_pool: class.cr.constant_pool.clone(),
            operand_stack: Vec::new(),
            local_variables: new_local_variables
        });

    }

    runtime.previous_frames.push(runtime.current_frame.clone());
    runtime.current_frame = new_frame.unwrap();
    try!(do_run_method(&mut runtime, &code.unwrap(), 0));
    return Ok(());
}

fn ifcmp<F>(desc: &str, mut runtime: &mut Runtime, mut buf: &mut Cursor<&Vec<u8>>, cmp: F) -> Result<(), RunnerError>
    where F: Fn(i32) -> bool
{
    let current_position = buf.position() - 1;
    let branch_offset = try!(buf.read_u16::<BigEndian>()) as i16;
    let popped = runtime.current_frame.operand_stack.pop().unwrap();
    debugPrint!(true, 2, "{} {} {}", desc, popped, branch_offset);
    if cmp(popped.to_int()) {
        let new_position = (current_position as i64 + branch_offset as i64) as u64;
        debugPrint!(true, 2, "BRANCHED from {} to {}", current_position, new_position);
        buf.set_position(new_position);
    }
    return Ok(());
}


fn icmp<F>(desc: &str, mut runtime: &mut Runtime, mut buf: &mut Cursor<&Vec<u8>>, cmp: F) -> Result<(), RunnerError>
    where F: Fn(i32, i32) -> bool
{
    let current_position = buf.position() - 1;
    let branch_offset = try!(buf.read_u16::<BigEndian>()) as i16;
    let popped2 = runtime.current_frame.operand_stack.pop().unwrap();
    let popped1 = runtime.current_frame.operand_stack.pop().unwrap();
    debugPrint!(true, 2, "{} {} {} {}", desc, popped1, popped2, branch_offset);
    if cmp(popped1.to_int(), popped2.to_int()) {
        let new_position = (current_position as i64 + branch_offset as i64) as u64;
        debugPrint!(true, 2, "BRANCHED from {} to {}", current_position, new_position);
        buf.set_position(new_position);
    }
    return Ok(());
}

fn do_run_method(mut runtime: &mut Runtime, code: &Code, pc: u16) -> Result<(), RunnerError> {
    if pc as usize > code.code.len() {
        return Err(RunnerError::InvalidPc);
    }
    let mut buf = Cursor::new(&code.code);

    loop {
        let current_position = buf.position();
        let op_code = try!(buf.read_u8());
        debugPrint!(true, 3, "{} Op code {}", runtime.count, op_code);
        runtime.count+=1;
        match op_code {
            1 => {
                debugPrint!(true, 2, "ACONST_NULL");
                // Bit weird, use a random class as the type. Probably need a special case for untyped null?
                runtime.current_frame.operand_stack.push(Variable::Reference(runtime.classes.values().nth(0).unwrap().clone(), None));
            }
            2...8 => {
                let val = (op_code as i32) - 3;
                debugPrint!(true, 2, "ICONST {}", val);
                runtime.current_frame.operand_stack.push(Variable::Int(val));
            }
            16 => {
                let byte = try!(buf.read_u8()) as i32;
                debugPrint!(true, 2, "BIPUSH {}", byte);
                runtime.current_frame.operand_stack.push(Variable::Int(byte));
            }
            17 => {
                let short = try!(buf.read_u16::<BigEndian>()) as i32;
                debugPrint!(true, 2, "SIPUSH {}", short);
                runtime.current_frame.operand_stack.push(Variable::Int(short));
            }
            18 => { // LDC
                let index = try!(buf.read_u8());
                let maybe_cp_entry = runtime.current_frame.constant_pool.get(&(index as u16)).map(|x| x.clone());
                if maybe_cp_entry.is_none() {
                    debugPrint!(true, 1, "LDC failed at index {}", index);
                    return Err(RunnerError::ClassInvalid("Error"));
                } else {
                    match maybe_cp_entry.as_ref().unwrap() {
                        &ConstantPoolItem::CONSTANT_String { index } => {
                            let mut arguments : Vec<Variable> = Vec::new();
                            let var = try!(construct_object(&mut runtime.classes, &"java/lang/String", &runtime.class_paths));
                            {
                                let str = try!(get_cp_str(&runtime.current_frame.constant_pool, index));
                                debugPrint!(true, 2, "LDC string {}", str);
                                arguments = vec!(var.clone(), construct_char_array(str));
                            }
                            let obj = try!(var.to_ref().ok_or(RunnerError::NullPointerException));
                            try!(invoke_manual(runtime, obj, arguments, "<init>", "([C)V"));

                            runtime.current_frame.operand_stack.push(var);
                        }
                        &ConstantPoolItem::CONSTANT_Integer { value } => {
                            debugPrint!(true, 2, "LDC int {}", value as i32);
                            runtime.current_frame.operand_stack.push(Variable::Int(value as i32));
                        }
                        _ => return Err(RunnerError::ClassInvalid2(format!("Unknown constant {:?}", maybe_cp_entry.as_ref().unwrap())))
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
            46 => try!(aload("IALOAD", runtime, Variable::Int)),
            47 => try!(aload("LALOAD", runtime, Variable::Long)),
            48 => try!(aload("FALOAD", runtime, Variable::Float)),
            49 => try!(aload("DALOAD", runtime, Variable::Double)),
            50 => try!(aload("AALOAD", runtime, Variable::Reference)),
            51 => try!(aload("BALOAD", runtime, Variable::Byte)),
            52 => try!(aload("CALOAD", runtime, Variable::Char)),
            53 => try!(aload("SALOAD", runtime, Variable::Short)),
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
            79 => try!(astore("IASTORE", runtime, Variable::Int)),
            80 => try!(astore("LASTORE", runtime, Variable::Long)),
            81 => try!(astore("FASTORE", runtime, Variable::Float)),
            82 => try!(astore("DASTORE", runtime, Variable::Double)),
            83 => try!(astore("AASTORE", runtime, Variable::Reference)),
            84 => try!(astore("BASTORE", runtime, Variable::Byte)),
            85 => try!(astore("CASTORE", runtime, Variable::Char)),
            86 => try!(astore("SASTORE", runtime, Variable::Short)),
            87 => {
                let popped = runtime.current_frame.operand_stack.pop().unwrap();
                debugPrint!(true, 2, "POP {}", popped);
            }
            88 => {
                let popped = runtime.current_frame.operand_stack.pop().unwrap();
                if popped.is_type_1() {
                    let popped2 = runtime.current_frame.operand_stack.pop().unwrap();
                    debugPrint!(true, 2, "POP2 {} {}", popped, popped2);
                } else {
                    debugPrint!(true, 2, "POP2 {}", popped);
                }
            }
            89 => {
                let stack_len = runtime.current_frame.operand_stack.len();
                let peek = runtime.current_frame.operand_stack[stack_len - 1].clone();
                debugPrint!(true, 2, "DUP {}", peek);
                runtime.current_frame.operand_stack.push(peek);
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
            136 => single_pop_instr("L2I", runtime, Variable::Int, Variable::to_long, |x| x as i32),
            147 => {
                let popped = runtime.current_frame.operand_stack.pop().unwrap();
                debugPrint!(true, 2, "I2S {}", popped);
                runtime.current_frame.operand_stack.push(Variable::Short(popped.to_int() as i16));
            }
            153 => try!(ifcmp("IFEQ", runtime, &mut buf, |x| x == 0)),
            154 => try!(ifcmp("IFNE", runtime, &mut buf, |x| x != 0)),
            155 => try!(ifcmp("IFLT", runtime, &mut buf, |x| x < 0)),
            156 => try!(ifcmp("IFGE", runtime, &mut buf, |x| x >= 0)),
            157 => try!(ifcmp("IFGT", runtime, &mut buf, |x| x > 0)),
            158 => try!(ifcmp("IFLE", runtime, &mut buf, |x| x <= 0)),
            159 => try!(ifcmp("IFEQ", runtime, &mut buf, |x| x == 0)),
            160 => try!(icmp("IF_ICMPNE", runtime, &mut buf, |x,y| x != y)),
            161 => try!(icmp("IF_ICMPLT", runtime, &mut buf, |x,y| x < y)),
            162 => try!(icmp("IF_ICMPGE", runtime, &mut buf, |x,y| x >= y)),
            163 => try!(icmp("IF_ICMPGT", runtime, &mut buf, |x,y| x > y)),
            164 => try!(icmp("IF_ICMPLE", runtime, &mut buf, |x,y| x <= y)),
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
                debugPrint!(true, 2, "Return");
                runtime.current_frame = runtime.previous_frames.pop().unwrap();
                return Ok(());
            }
            178 => { // getstatic
                let index = try!(buf.read_u16::<BigEndian>());
                let (class_name, field_name, typ) = try!(get_cp_field(&runtime.current_frame.constant_pool, index));
                debugPrint!(true, 2, "GETSTATIC {} {} {}", class_name, field_name, typ);
                let class_result = try!(load_class(&mut runtime.classes, class_name, &runtime.class_paths));
                let statics = class_result.statics.borrow();
                let maybe_static_variable = statics.get(field_name);
                if maybe_static_variable.is_none() {
                    return Err(RunnerError::ClassNotLoaded(String::from(class_name)));
                }
                runtime.current_frame.operand_stack.push(maybe_static_variable.unwrap().clone());
            }
            180 => {
                let field_index = try!(buf.read_u16::<BigEndian>());
                let (class_name, field_name, typ) = try!(get_cp_field(&runtime.current_frame.constant_pool, field_index));
                let var = runtime.current_frame.operand_stack.pop().unwrap();
                let obj = try!(try!(get_obj_instance_from_variable(&var)).ok_or(RunnerError::NullPointerException));
                debugPrint!(true, 2, "GETFIELD class:'{}' field:'{}' type:'{}' object:'{}'", class_name, field_name, typ, obj);
                let mut super_obj = try!(get_super_obj(obj, class_name));
                let mut super_obj_with_field = try!(get_obj_field(super_obj, field_name));
                let members = super_obj_with_field.members.borrow();
                runtime.current_frame.operand_stack.push(members.get(field_name).unwrap().clone());
            }
            181 => {
                let field_index = try!(buf.read_u16::<BigEndian>());
                let (class_name, field_name, typ) = try!(get_cp_field(&runtime.current_frame.constant_pool, field_index));
                let value = runtime.current_frame.operand_stack.pop().unwrap();
                let var = runtime.current_frame.operand_stack.pop().unwrap();
                let obj = try!(try!(get_obj_instance_from_variable(&var)).ok_or(RunnerError::NullPointerException));
                debugPrint!(true, 2, "PUTFIELD {} {} {} {} {}", class_name, field_name, typ, obj, value);
                let mut super_obj = try!(get_super_obj(obj, class_name));
                let mut super_obj_with_field = try!(get_obj_field(super_obj, field_name));
                let mut members = super_obj_with_field.members.borrow_mut();
                members.insert(String::from(field_name), value);
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
                let count = try!(buf.read_u8());
                let _zero = try!(buf.read_u8());
                try!(invoke("INVOKEINTERFACE", runtime, index, true, false));
            }
            187 => {
                let index = try!(buf.read_u16::<BigEndian>());
                let class_name = try!(get_cp_class(&runtime.current_frame.constant_pool, index));
                debugPrint!(true, 2, "NEW {}", class_name);
                let var = try!(construct_object(&mut runtime.classes, &class_name, &runtime.class_paths));
                runtime.current_frame.operand_stack.push(var);
            }
            188 => {
                let atype = try!(buf.read_u8());
                let count = try!(runtime.current_frame.operand_stack.pop().ok_or(RunnerError::ClassInvalid("Error"))).to_int();
                debugPrint!(true, 2, "NEWARRAY {} {}", atype, count);
                let mut v : Vec<Variable> = Vec::new();
                for c in 0..count {
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
                            _ => return Err(RunnerError::ClassInvalid("Error"))
                        });
                }
                runtime.current_frame.operand_stack.push(Variable::ArrayReference(Rc::new(v[0].clone()), Some(Rc::new(RefCell::new(v)))));
            }
            190 => {
                let var = runtime.current_frame.operand_stack.pop().unwrap();
                let (typee, array) = var.to_arrayref();
                if array.is_none() {
                    return Err(RunnerError::NullPointerException);
                }
                let len = array.as_ref().unwrap().borrow().len();
                debugPrint!(true, 2, "ARRAYLEN {} {} {}", var, typee, len);
                runtime.current_frame.operand_stack.push(Variable::Int(len as i32));
            }
            192 => {
                let var = runtime.current_frame.operand_stack.pop().unwrap();
                let index = try!(buf.read_u16::<BigEndian>());

                debugPrint!(true, 2, "CHECKCAST {} {}", var, index);

                let maybe_cp_entry = runtime.current_frame.constant_pool.get(&index);
                if maybe_cp_entry.is_none() {
                    debugPrint!(true, 1, "Missing CP class {}", index);
                    return Err(RunnerError::ClassInvalid("Error"));
                } else {
                    // TODO: CHECKCAST (noop)
                    runtime.current_frame.operand_stack.push(var);
                }
            }
            194 => {
                let var = runtime.current_frame.operand_stack.pop().unwrap();
                debugPrint!(true, 2, "MONITORENTER {}", var);
                let obj = try!(try!(get_obj_instance_from_variable(&var)).ok_or(RunnerError::NullPointerException));
                // TODO: Implement monitor
                debugPrint!(true, 1, "WARNING: MonitorEnter not implemented");
            },
            199 => {
                let branch_offset = try!(buf.read_u16::<BigEndian>()) as i16;
                let var = runtime.current_frame.operand_stack.pop().unwrap();
                debugPrint!(true, 2, "IFNONNULL {} {}", var, branch_offset);
                let maybe_obj = try!(get_obj_instance_from_variable(&var));
                if maybe_obj.is_some() {
                    let new_pos = (current_position as i64 + branch_offset as i64) as u64;
                    debugPrint!(true, 2, "BRANCHED from {} to {}", current_position, new_pos);
                    buf.set_position(new_pos);
                }
            }
            _ => return Err(RunnerError::UnknownOpCode(op_code))
        }
    }
}

fn find_class(name: &str, class_paths: &Vec<String>) -> Result<ClassResult, RunnerError> {
    debugPrint!(true, 3, "Finding class {}", name);
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
        debugPrint!(true, 3, "Finding class {} direct load failed, searching {}", name, class_path);

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
    debugPrint!(true, 2, "Bootstrap totally complete on {}", name);
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

    if class_result.super_class_index > 0 {
        let class_name = String::from(try!(get_cp_class(&class_result.constant_pool, class_result.super_class_index)));
        if !classes.contains_key(&class_name) {
            unresolved_classes.insert(class_name);
        }
    }
    if !classes.contains_key(&String::from("java/lang/Object")) {
        unresolved_classes.insert(String::from("java/lang/Object"));
    }
    return Ok(());
}

fn initialise_class(classes: &mut HashMap<String, Rc<Class>>, class: &Rc<Class>) -> Result<(), RunnerError> {
    debugPrint!(true, 2, "Initialising class {}", class.name);
    if *class.initialised.borrow() {
        return Ok(());
    }

    for field in &class.cr.fields {
        if field.access_flags & ACC_STATIC == 0 {
            continue;
        }

        let name_string = try!(get_cp_str(&class.cr.constant_pool, field.name_index));
        let descriptor_string = try!(get_cp_str(&class.cr.constant_pool, field.descriptor_index));

        debugPrint!(true, 3, "Constructing class static member {} {}", name_string, descriptor_string);

        let var = try!(initialise_variable(classes, descriptor_string));

        class.statics.borrow_mut().insert(String::from(name_string), var);
    }
    if class.cr.super_class_index > 0 {
        let super_class_name = String::from(try!(get_cp_class(&class.cr.constant_pool, class.cr.super_class_index)));
        debugPrint!(true, 3, "Class {} has superclass {}", class.name, super_class_name);
        *class.super_class.borrow_mut() = Some(try!(classes.get(&super_class_name).ok_or(RunnerError::ClassInvalid("Error"))).clone());
    } else {
        if class.name != "java/lang/Object" {
            debugPrint!(true, 3, "Class {} has superclass {}", class.name, "java/lang/Object");
            *class.super_class.borrow_mut() = Some(try!(classes.get(&String::from("Java/lang/Object")).ok_or(RunnerError::ClassInvalid("Error"))).clone());
        }
    }
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

fn generate_method_descriptor(args: &Vec<Variable>, return_type: Option<&Variable>) -> String {
    let mut ret = String::new();
    ret.push('(');
    for arg in args {
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

fn parse_single_type_string(classes: &HashMap<String, Rc<Class>>, string: &str) -> Result<Variable, RunnerError> {
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
            return Err(RunnerError::ClassInvalid("Error"));
        }
    }

    return Ok(variable);
}

fn parse_function_type_string(classes: &HashMap<String, Rc<Class>>, string: &str) -> Result<(Vec<Variable>, Option<Variable>), RunnerError> {
    let debug = false;
    let mut iter = string.chars().peekable();

    if iter.next().unwrap_or(' ') != '(' {
        debugPrint!(true, 2, "Type {} invalid", string);
        return Err(RunnerError::ClassInvalid("Error"));
    }

    let mut parameters : Vec<Variable> = Vec::new();
    let mut type_char : char = '\0';
    while {type_char = try!(iter.next().ok_or(RunnerError::ClassInvalid("Error"))); type_char != ')'} {
        let mut type_string = String::new();
        while type_char == '[' {
            type_string.push(type_char);
            type_char = try!(iter.next().ok_or(RunnerError::ClassInvalid("Error")));
        }
        type_string.push(type_char);

        if type_char == 'L' {
            type_string.push_str(iter.by_ref().take_while(|x| *x != ';').collect::<String>().as_str());
        }
        debugPrint!(debug, 3, "Found parameter {}", type_string);
        parameters.push(try!(parse_single_type_string(classes, type_string.as_str())));
        debugPrint!(debug, 3, "Parameters now {:?}", parameters);
    }

    let return_type_string : String = iter.collect();
    debugPrint!(debug, 3, "Return type {}", return_type_string);
    if return_type_string == "V" {
        return Ok((parameters, None));
    } else {
        return Ok((parameters, Some(try!(parse_single_type_string(classes, return_type_string.as_str())))));
    }
}

pub fn run(class_paths: &Vec<String>, class: &ClassResult) -> Result<(), RunnerError> {
    println!("Running");
    let mut runtime = Runtime {
        class_paths: class_paths.clone(),
        previous_frames: Vec::new(),
        current_frame: Frame {
            constant_pool: class.constant_pool.clone(),
            operand_stack: Vec::new(),
            local_variables: Vec::new()},
        classes: HashMap::new(),
        count: 0
    };

    bootstrap_class_and_dependencies(&mut runtime.classes, String::new().as_str(), class, class_paths);

    let main_code = try!(get_class_method_code(class, &"main", &"([Ljava/lang/String;)V"));

    try!(do_run_method(&mut runtime, &main_code, 0));

    return Ok(());
}

pub fn run_method(class_paths: &Vec<String>, class: &ClassResult, method: &str, arguments: &Vec<Variable>, return_type: Option<&Variable>) -> Result<Variable, RunnerError> {
    println!("Running method {} with {} arguments", method, arguments.len());
    let mut runtime = Runtime {
        class_paths: class_paths.clone(),
        previous_frames: vec!(Frame {
            constant_pool: HashMap::new(),
            operand_stack: Vec::new(),
            local_variables: Vec::new()}),
        current_frame: Frame {
            constant_pool: class.constant_pool.clone(),
            operand_stack: Vec::new(),
            local_variables: Vec::new()},
        classes: HashMap::new(),
        count: 0
    };

    bootstrap_class_and_dependencies(&mut runtime.classes, String::new().as_str(), class, class_paths);

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

    let method_descriptor = generate_method_descriptor(&arguments, return_type);
    debugPrint!(true, 1, "Finding method {} with descriptor {}", method, method_descriptor);
    let code = try!(get_class_method_code(class, method, method_descriptor.as_str()));

    println!("Running method");
    try!(do_run_method(&mut runtime, &code, 0));

    return Ok(runtime.current_frame.operand_stack.pop().unwrap().clone());
}