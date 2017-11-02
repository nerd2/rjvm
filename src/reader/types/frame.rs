use reader::class_reader::*;
use reader::runner::*;
use std::collections::HashMap;
use std::rc::Rc;

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
