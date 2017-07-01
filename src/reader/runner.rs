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

#[derive(Clone, Debug, PartialEq)]
pub struct Class {
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

#[derive(Clone, Debug, PartialEq)]
pub struct Object {
    typeRef: Rc<Class>,
    members: HashMap<String, Variable>,
}
impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Object type:{}", self.typeRef.name)
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
    ArrayReference(Rc<Variable>, Option<Rc<Vec<Variable>>>), // First argument is dummy for array type
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

fn get_class_method_code(class: &ClassResult, target_method_name: &str, target_descriptor: &str) -> Result<Code, RunnerError> {
    let mut method_res: Result<&FieldItem, RunnerError> = Err(RunnerError::ClassInvalid);

    for method in &class.methods {
        let method_name = try!(get_cp_str(&class.constant_pool, method.name_index));
        let descriptor = try!(get_cp_str(&class.constant_pool, method.descriptor_index));
        debugPrint!(true, 3, "Checking method {} {}", method_name, descriptor);
        if method_name == target_method_name &&
            descriptor == target_descriptor {
            method_res = Ok(method);
            break;
        }
    }

    let method = try!(method_res);
    debugPrint!(true, 3, "Found method");
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

fn load<F>(desc: &str, index: u8, mut runtime: &mut Runtime, t: F) { // TODO: Type checking
    let loaded = runtime.current_frame.local_variables[index as usize].clone();
    debugPrint!(true, 2, "{} {} {}", desc, index, loaded);
    runtime.current_frame.operand_stack.push(loaded);
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

fn neg<F, G, K>(desc: &str, mut runtime: &mut Runtime, creator: F, extractor: G)
    where
        F: Fn(<K as std::ops::Sub>::Output) -> Variable,
        G: Fn(&Variable) -> K,
        K: Sub,
        K: Default
{
    let popped = runtime.current_frame.operand_stack.pop().unwrap();
    debugPrint!(true, 2, "{} {}", desc, popped);
    runtime.current_frame.operand_stack.push(creator(<K as Default>::default() - extractor(&popped)));
}


fn vreturn<F, K>(desc: &str, mut runtime: &mut Runtime, extractor: F) -> Result<(), RunnerError> where F: Fn(&Variable) -> K {
    let popped = runtime.current_frame.operand_stack.pop().unwrap();
    debugPrint!(true, 2, "{} {}", desc, popped);
    extractor(&popped); // Type check
    runtime.current_frame = runtime.previous_frames.pop().unwrap();
    runtime.current_frame.operand_stack.push(popped);
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
            21 => load("ILOAD", try!(buf.read_u8()), runtime, Variable::Int),
            22 => load("LLOAD", try!(buf.read_u8()), runtime, Variable::Long),
            23 => load("FLOAD", try!(buf.read_u8()), runtime, Variable::Float),
            24 => load("DLOAD", try!(buf.read_u8()), runtime, Variable::Double),
            25 => load("ALOAD", try!(buf.read_u8()), runtime, Variable::Reference),
            26...29 => load("ILOAD", op_code - 26, runtime, Variable::Int),
            30...33 => load("LLOAD", op_code - 30, runtime, Variable::Long),
            34...37 => load("LLOAD", op_code - 34, runtime, Variable::Float),
            38...41 => load("DLOAD", op_code - 38, runtime, Variable::Double),
            42...45 => load("ALOAD", op_code - 42, runtime, Variable::Reference),
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
            116 => neg("INEG", runtime, Variable::Int, Variable::to_int),
            117 => neg("LNEG", runtime, Variable::Long, Variable::to_long),
            118 => neg("FNEG", runtime, Variable::Float, Variable::to_float),
            119 => neg("DNEG", runtime, Variable::Double, Variable::to_double),
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
            136 => single_pop_instr("L2I", runtime, Variable::Int, Variable::to_long, |x| x as i32),
            147 => {
                let popped = runtime.current_frame.operand_stack.pop().unwrap();
                debugPrint!(true, 2, "I2S {}", popped);
                runtime.current_frame.operand_stack.push(Variable::Short(popped.to_int() as i16));
            }
            172 => { return vreturn("IRETURN", runtime, Variable::to_int); }
            173 => { return vreturn("LRETURN", runtime, Variable::to_long); }
            174 => { return vreturn("FRETURN", runtime, Variable::to_float); }
            175 => { return vreturn("DRETURN", runtime, Variable::to_double); }
            176 => { return vreturn("ARETURN", runtime, Variable::to_ref); }
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
                try!(do_run_method(&mut runtime, &code.unwrap(), 0));
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

fn generate_variable_descriptor(var: &Variable) -> String {
    let mut ret = String::new();
    match var {
        &Variable::Byte(v) => {ret.push('B');},
        &Variable::Char(v) => {ret.push('C');},
        &Variable::Double(v) => {ret.push('D');},
        &Variable::Float(v) => {ret.push('F');},
        &Variable::Int(v) => {ret.push('I');},
        &Variable::Long(v) => {ret.push('J');},
        &Variable::Short(v) => {ret.push('S');},
        &Variable::Boolean(v) => {ret.push('Z');},
        &Variable::Reference(ref class, ref obj) => {
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

pub fn run(class_paths: &Vec<String>, class: &ClassResult) -> Result<(), RunnerError> {
    println!("Running");
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
        classes: HashMap::new()
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