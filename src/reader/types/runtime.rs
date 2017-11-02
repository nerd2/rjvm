extern crate rand;
use reader::runner::*;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Runtime {
    pub previous_frames: Vec<Frame>,
    pub current_frame: Frame,
    pub class_paths: Vec<String>,
    pub classes: HashMap<String, Rc<Class>>,
    pub count: i64,
    pub current_thread: Option<Variable>,
    pub string_interns: HashMap<String, Variable>,
    pub properties: HashMap<String, Variable>,
    pub class_objects: HashMap<String, Variable>,
    pub object_count: i32,
}
impl Runtime {
    pub fn new(class_paths: Vec<String>) -> Runtime {
        return Runtime {
            class_paths: class_paths,
            previous_frames: vec!(Frame::new()),
            current_frame: Frame::new(),
            classes: HashMap::new(),
            count: 0,
            current_thread: None,
            string_interns: HashMap::new(),
            properties: HashMap::new(),
            class_objects: HashMap::new(),
            object_count: rand::random::<i32>(),
        };
    }

    pub fn reset_frames(&mut self) {
        self.previous_frames = vec!(Frame::new());
        self.current_frame = Frame::new();
    }

    pub fn get_next_object_code(&mut self) -> i32 {
        let ret = self.object_count;
        self.object_count += 1;
        return ret;
    }

    pub fn push_on_stack(&mut self, var: Variable) {
        if !var.is_type_1() {
            self.current_frame.operand_stack.push(var.clone());
        }
        self.current_frame.operand_stack.push(var);
    }
}