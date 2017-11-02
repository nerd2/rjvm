use reader::jvm::construction::*;
use reader::runner::*;
use reader::util::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::rc::Weak;

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

pub fn get_most_sub_class(mut obj: Rc<Object>) -> Rc<Object>{
    // Go to top of chain
    while obj.sub_class.borrow().is_some() {
        let new_obj = obj.sub_class.borrow().as_ref().unwrap().upgrade().unwrap();
        obj = new_obj;
    }
    return obj;
}

pub fn get_super_obj(mut obj: Rc<Object>, class_name: &str) -> Result<Rc<Object>, RunnerError> {
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

// Get the (super)object which contains a field
pub fn get_obj_field(mut obj: Rc<Object>, field_name: &str) -> Result<Rc<Object>, RunnerError> {
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
