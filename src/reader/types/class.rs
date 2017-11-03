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
    pub super_class: RefCell<Option<Rc<Class>>>
}
impl Class {
    pub fn new(name: &String, cr: &ClassResult) -> Class {
        return Class { name: name.clone(), initialising: RefCell::new(false), initialised: RefCell::new(false), cr: cr.clone(), statics: RefCell::new(HashMap::new()), super_class: RefCell::new(None)};
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
}
