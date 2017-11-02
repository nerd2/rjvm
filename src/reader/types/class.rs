use reader::class_reader::*;
use reader::types::variable::*;
use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

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
