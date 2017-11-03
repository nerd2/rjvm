use reader::class_reader::*;
use reader::runner::*;
use reader::util::*;
use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;
use std::rc::Weak;

pub fn initialise_variable(runtime: &mut Runtime, descriptor_string: &str) -> Result<Variable, RunnerError> {
    let variable = try!(parse_single_type_descriptor(runtime, descriptor_string, false));
    return Ok(variable);
}

pub fn construct_char_array(runtime: &mut Runtime, s: &str) -> Variable {
    let mut v : Vec<Variable> = Vec::new();
    for c in s.chars() {
        v.push(Variable::Char(c));
    }
    let array_object = ArrayObject {
        is_null: false,
        element_type_ref: None,
        element_type_str: String::from("C"),
        elements: RefCell::new(v),
        code: runtime.get_next_object_code()
    };
    return Variable::ArrayReference(Rc::new(array_object));
}

pub fn construct_array(runtime: &mut Runtime, class: Rc<Class>, data: Option<Vec<Variable>>) -> Result<Variable, RunnerError> {
    let array_object = ArrayObject {
        is_null: data.is_none(),
        element_type_ref: Some(class.clone()),
        element_type_str: generate_class_descriptor(&class),
        elements: RefCell::new(data.unwrap_or(Vec::new())),
        code: runtime.get_next_object_code()
    };
    return Ok(Variable::ArrayReference(Rc::new(array_object)));
}

pub fn construct_array_by_name(runtime: &mut Runtime, name: &str, data: Option<Vec<Variable>>) -> Result<Variable, RunnerError> {
    let class = try!(load_class(runtime, name));
    return construct_array(runtime, class, data);
}

pub fn construct_primitive_array(runtime: &mut Runtime, element_type: &str, data: Option<Vec<Variable>>) -> Result<Variable, RunnerError> {
    // TODO
    let array_object = ArrayObject {
        is_null: data.is_none(),
        element_type_ref: None,
        element_type_str: String::from(element_type),
        elements: RefCell::new(data.unwrap_or(Vec::new())),
        code: runtime.get_next_object_code()
    };
    return Ok(Variable::ArrayReference(Rc::new(array_object)));
}

pub fn construct_null_object(runtime: &mut Runtime, class: Rc<Class>) -> Result<Variable, RunnerError> {
    let obj = Rc::new(Object {
        is_null: true,
        type_ref: class,
        members: RefCell::new(HashMap::new()),
        super_class: RefCell::new(None),
        sub_class: RefCell::new(None),
        code: runtime.get_next_object_code()
    });
    return Ok(Variable::Reference(obj));
}

pub fn construct_null_object_by_name(runtime: &mut Runtime, name: &str) -> Result<Variable, RunnerError> {
    return parse_single_type_descriptor(runtime, name, true);
}

pub fn construct_object(runtime: &mut Runtime, name: &str) -> Result<Variable, RunnerError> {
    let debug = false;
    runnerPrint!(runtime, debug, 3, "Constructing object {}", name);
    try!(load_class(runtime, name));

    let original_class = try!(load_class(runtime, name));
    let mut original_obj : Option<Rc<Object>> = None;
    let mut class = original_class.clone();
    let mut sub_class : Option<Weak<Object>> = None;

    loop {
        runnerPrint!(runtime, debug, 3, "Constructing object of type {} with subclass {}", class.name, sub_class.is_some());
        let mut members: HashMap<String, Variable> = HashMap::new();
        for field in &class.cr.fields {
            if field.access_flags & ACC_STATIC != 0 {
                continue;
            }

            let name_string = try!(class.cr.constant_pool.get_str(field.name_index));
            let descriptor_string = try!(class.cr.constant_pool.get_str(field.descriptor_index));

            let var = try!(initialise_variable(runtime, descriptor_string.as_str()));

            members.insert((*name_string).clone(), var);
        }

        let obj = Rc::new(Object {
            is_null: false,
            type_ref: class.clone(),
            members: RefCell::new(members),
            super_class: RefCell::new(None),
            sub_class: RefCell::new(sub_class.clone()),
            code: runtime.get_next_object_code()
        });
        if original_obj.is_none() {
            original_obj = Some(obj.clone());
        }
        if sub_class.is_some() {
            let sub_class_up = sub_class.unwrap().upgrade().unwrap();
            *sub_class_up.super_class.borrow_mut() = Some(obj.clone());
        }
        let maybe_super_class = class.super_class.borrow().clone();
        if maybe_super_class.is_some() {
            sub_class = Some(Rc::downgrade(&obj.clone()));
            class = maybe_super_class.unwrap();
        } else {
            return Ok(Variable::Reference(original_obj.unwrap()));
        }
    }
}
