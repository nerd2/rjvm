use reader::jvm::construction::*;
use reader::runner::*;
use reader::util::*;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::fmt;
use std::boxed::Box;
use std::rc::Rc;

#[derive(Debug)]
pub struct Object {
    code: i32,
    type_ref: Rc<Class>,
    members: RefCell<Box<[Variable]>>
}

impl Object {
    pub fn new(runtime: &mut Runtime, type_ref: &Rc<Class>, num_members: usize) -> Rc<Object> {
        let obj = Object {
            code: runtime.get_next_object_code(),
            type_ref: type_ref.clone(),
            members: RefCell::new(vec![Variable::Boolean(false); num_members].into_boxed_slice()),
        };
        return Rc::new(obj);
    }

    pub fn type_ref(&self) -> Rc<Class> {
        return self.type_ref.clone();
    }

    pub fn code(&self) -> i32 {
        return self.code;
    }

    pub fn get_member(&self, name: &String) -> Option<Variable> {
        let maybe_offset = self.type_ref().find_member_offset(name);
        if maybe_offset.is_some() {
            return self.get_member_at_offset(maybe_offset.unwrap());
        } else {
            return None;
        }
    }

    pub fn get_member_at_offset(&self, offset: usize) -> Option<Variable> {
        return Some(self.members.borrow()[offset].clone());
    }

    pub fn put_member(&self, name: &String, var: Variable) -> Option<()> {
        let maybe_offset = self.type_ref().find_member_offset(name);
        return maybe_offset.map(|x| self.put_member_at_offset(x, var));
    }

    pub fn put_member_at_offset(&self, offset: usize, var: Variable) {
        self.members.borrow_mut()[offset] = var;
    }

    pub fn deep_compare(&self, other:&Self) -> bool {
        if self.type_ref != other.type_ref {
            return false;
        }

        for i in 0..*self.type_ref.total_size.borrow() {
            if self.get_member_at_offset(i) != other.get_member_at_offset(i) {
                return false;
            }
        }

        return true;
    }
}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return match self.type_ref.name.as_str() {
            "java/lang/String" => {
                let str = string_to_string(self);
                write!(f, "String {} '{}'", self.code, str.as_str())
            }
            _ => {write!(f, "Object {} type:{}",self.code, self.type_ref.name.as_str()) }
        }
    }
}

impl Hash for Object {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.code.hash(state);
    }
}

impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        return self as *const _ == other as *const _;
    }
}

impl Eq for Object {}

pub fn put_static(runtime: &mut Runtime, class_name: &str, field_name: &str, value: Variable) -> Result<(), RunnerError> {
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

fn null_check(runtime: &mut Runtime, obj: &Option<Rc<Object>>) -> Result<(), RunnerError> {
    if obj.is_none() {
        let exception = try!(construct_object(runtime, &"java/lang/NullPointerException"));
        return Err(RunnerError::Exception(exception));
    }
    return Ok(());
}

pub fn put_field(runtime: &mut Runtime, obj: &Option<Rc<Object>>, field_name: &str, value: Variable) -> Result<(), RunnerError> {
    try!(null_check(runtime, obj));
    let type_ref = obj.as_ref().unwrap().type_ref().clone();
    return put_field_specific_class(runtime, obj, &type_ref, field_name, value);
}

pub fn put_field_specific_class_name(runtime: &mut Runtime, obj: &Option<Rc<Object>>, class_name: &str, field_name: &str, value: Variable) -> Result<(), RunnerError> {
    try!(null_check(runtime, obj));
    let class = try!(load_class(runtime, class_name));
    return put_field_specific_class(runtime, obj, &class, field_name, value);
}

pub fn put_field_specific_class(runtime: &mut Runtime, obj: &Option<Rc<Object>>, class: &Rc<Class>, field_name: &str, value: Variable) -> Result<(), RunnerError> {
    let debug = false;
    runnerPrint!(runtime, debug, 2, "Put Field Specific Class {} {} {}", class.name, field_name, value);

    let maybe_offset = class.find_member_offset(&String::from(field_name));
    if maybe_offset.is_none() {
        panic!("TODO, class doesn't contain field");
    }

    obj.as_ref().unwrap().put_member_at_offset(maybe_offset.unwrap(), value);
    return Ok(());
}

pub fn get_field(runtime: &mut Runtime, obj: &Option<Rc<Object>>, class_name: &str, field_name: &str) -> Result<Variable, RunnerError> {
    try!(null_check(runtime, obj));

    let debug = false;

    runnerPrint!(runtime, debug, 2, "Get Field {} {} {}", *obj.as_ref().unwrap(), class_name, field_name);

    let class = try!(load_class(runtime, class_name));
    let maybe_offset = class.find_member_offset(&String::from(field_name));
    if maybe_offset.is_none() {
        panic!("TODO, class doesn't contain field");
    }

    let member = obj.as_ref().unwrap().get_member_at_offset(maybe_offset.unwrap()).unwrap();
    if !member.is_unresolved() {
        return Ok(member.clone());
    }
    let var = try!(construct_null_object_by_name(runtime, member.get_unresolved_type_name().clone().as_str()));
    obj.as_ref().unwrap().put_member_at_offset(maybe_offset.unwrap(), var.clone());
    return Ok(var);
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
