extern crate rand;
extern crate zip;
use reader::class_reader::*;
use reader::runner::*;
use reader::builtins::*;
use std::collections::HashMap;
use std::rc::Rc;
use std::rc::Weak;
use std::fs::File;

pub struct Runtime {
    pub previous_frames: Vec<Frame>,
    pub current_frame: Frame,
    pub class_paths: Vec<String>,
    pub jars: Vec<zip::ZipArchive<File>>,
    pub classes: HashMap<String, Rc<Class>>,
    pub count: i64,
    pub current_thread: Option<Variable>,
    pub string_interns: HashMap<String, Variable>,
    pub properties: HashMap<String, Variable>,
    pub class_objects: HashMap<String, Variable>,
    pub object_count: i32,
    pub stdout: String,
    pub stderr: String,
    pub free_mem: i64,
    pub objects: Vec<Weak<Object>>
}
impl Runtime {
    pub fn new(class_paths: Vec<String>, jars: Vec<zip::ZipArchive<File>>) -> Runtime {
        return Runtime {
            class_paths: class_paths,
            jars: jars,
            previous_frames: vec!(Frame::new()),
            current_frame: Frame::new(),
            classes: HashMap::new(),
            count: 0,
            current_thread: None,
            string_interns: HashMap::new(),
            properties: HashMap::new(),
            class_objects: HashMap::new(),
            object_count: rand::random::<i32>(),
            stdout: String::new(),
            stderr: String::new(),
            free_mem: 4 * 1024 * 1024,
            objects: Vec::new()
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

    pub fn pop_from_stack(&mut self) -> Option<Variable> {
        let maybe_var = self.current_frame.operand_stack.pop();
        maybe_var.as_ref().map(|x| {if !x.is_type_1() {self.current_frame.operand_stack.pop();}});
        return maybe_var;
    }
    
    pub fn add_arguments(&mut self, arguments: &Vec<Variable>) {
        for arg in arguments {
            match arg {
                &Variable::Long(ref _x) => {
                    self.current_frame.local_variables.push(arg.clone());
                    self.current_frame.local_variables.push(arg.clone());
                },
                &Variable::Double(ref _x) => {
                    self.current_frame.local_variables.push(arg.clone());
                    self.current_frame.local_variables.push(arg.clone());
                },
                _ => {
                    self.current_frame.local_variables.push(arg.clone());
                }
            }
        }
    }

    pub fn invoke(&mut self, class_name: Rc<String>, method_name: Rc<String>, descriptor: Rc<String>, with_obj: bool, special: bool) -> Result<(), RunnerError> {
        let mut code : Option<Code>;
        let new_frame : Option<Frame>;
        let new_method_name : Option<String>;
        let current_op_stack_size = self.current_frame.operand_stack.len();

        new_method_name = Some((*class_name).clone() + "/" + method_name.as_str());
        let (parameters, _return_type) = try!(parse_function_type_descriptor(self, descriptor.as_str()));
        let extra_parameter = if with_obj {1} else {0};
        let new_local_variables = self.current_frame.operand_stack.split_off(current_op_stack_size - parameters.len() - extra_parameter);

        let mut class = try!(load_class(self, class_name.as_str()));

        if with_obj {

            if new_local_variables[0].is_null() {
                return Err(RunnerError::ClassInvalid2(format!("NULL obj ref on local var stack for method on {}", class_name)));
            }

            if new_local_variables[0].is_reference() {
                let obj = new_local_variables[0].to_ref();

                if !special {
                    class = obj.as_ref().unwrap().type_ref();
                }

                // Find method
                while {
                    code = class.cr.get_code(method_name.as_str(), descriptor.as_str()).ok();
                    code.is_none()
                } {
                    let new_class = class.super_class.borrow().clone();
                    if new_class.is_none() {
                        if try!(try_builtin(&class_name, &method_name, &descriptor, &new_local_variables, self)) {
                            return Ok(());
                        }

                        return Err(RunnerError::ClassInvalid2(format!("Could not find super class of object '{}' that matched method '{}' '{}'", obj.unwrap(), method_name, descriptor)))
                    }
                    class = new_class.unwrap();
                }
            } else if new_local_variables[0].is_array_reference() {
                let arrayobj = new_local_variables[0].to_arrayobj();
                // TODO, other "Object" methods like clone?
                if try!(try_builtin(&class_name, &method_name, &descriptor, &new_local_variables, self)) {
                    return Ok(());
                }

                return Err(RunnerError::ClassInvalid2(format!("Could not find super class of array '{}' that matched method '{}' '{}'", arrayobj, method_name, descriptor)))
            } else {
                panic!("Tried to invoke method on {}", new_local_variables[0]);
            }
        } else {
            if try!(try_builtin(&class_name, &method_name, &descriptor, &new_local_variables, self)) {
                return Ok(());
            }

            code = Some(try!(class.cr.get_code(method_name.as_str(), descriptor.as_str())));
        }

        new_frame = Some(Frame {
            class: Some(class.clone()),
            constant_pool: class.cr.constant_pool.clone(),
            operand_stack: Vec::new(),
            local_variables: new_local_variables,
            name: new_method_name.unwrap(),
            code: code.unwrap(),
            return_pos: 0,
        });

        self.previous_frames.push(self.current_frame.clone());
        self.current_frame = new_frame.unwrap();
        return Err(RunnerError::Invoke);
    }
}