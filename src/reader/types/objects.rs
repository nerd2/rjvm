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
