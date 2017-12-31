use reader::class_reader::*;
use reader::jvm::interpreter::invoke_nested;
use reader::runner::RunnerError;
use reader::types::variable::*;
use reader::types::runtime::Runtime;
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
    pub super_class: RefCell<Option<Rc<Class>>>,
    member_offset: RefCell<HashMap<Rc<String>, usize>>,
    pub total_size: RefCell<usize>,
}
impl Class {
    pub fn new(name: &String, cr: &ClassResult) -> Class {
        return Class {
            name: name.clone(),
            initialising: RefCell::new(false),
            initialised: RefCell::new(false),
            cr: cr.clone(),
            statics: RefCell::new(HashMap::new()),
            super_class: RefCell::new(None),
            member_offset: RefCell::new(HashMap::new()),
            total_size: RefCell::new(0)
        };
    }

    pub fn initialise(runtime: &mut Runtime, class: &Rc<Class>) -> Result<(), RunnerError> {
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

    pub fn find_member_offset(&self, name: &String) -> Option<usize> {
        let member_offset = self.member_offset.borrow();
        let maybe_my_offset = member_offset.get(name);
        if maybe_my_offset.is_some() {
            return Some(*maybe_my_offset.unwrap());
        }

        let mut class = self.super_class.borrow().clone();
        while class.is_some() {
            {
                let member_offset = class.as_ref().unwrap().member_offset.borrow();
                let maybe_offset = member_offset.get(name);
                if maybe_offset.is_some() {
                    return Some(*maybe_offset.unwrap());
                }
            }
            let new_class = class.as_ref().unwrap().super_class.borrow().clone();
            class = new_class;
        }

        return None;
    }

    pub fn find_superclass(mut class: Rc<Class>, name: Rc<String>) -> Option<Rc<Class>> {
        loop {
            if class.name == *name {
                return Some(class.clone());
            }

            let maybe_super_class = class.super_class.borrow().clone();

            if maybe_super_class.is_none() {
                return None;
            }

            class = maybe_super_class.as_ref().unwrap().clone();
        }
    }

    pub fn set_member_offset(&self, name: Rc<String>, offset: usize) {
        self.member_offset.borrow_mut().insert(name, offset);
    }
}
