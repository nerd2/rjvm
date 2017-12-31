use reader::class_reader::*;
use reader::jvm::gc::*;
use reader::runner::*;
use reader::util::*;
use std::cell::RefCell;
use std::mem::size_of;
use std::rc::Rc;

pub fn initialise_variable(runtime: &mut Runtime, descriptor_string: &str) -> Result<Variable, RunnerError> {
    let variable = try!(parse_single_type_descriptor(runtime, descriptor_string, false));
    return Ok(variable);
}

pub fn construct_char_array(runtime: &mut Runtime, s: &str) -> Variable {
    let mut v : Vec<Variable> = Vec::new();
    for c in s.chars() {
        v.push(Variable::Char(c));
    }
    runtime.free_mem -= v.len() as i64 + size_of::<(ArrayObject)>() as i64;
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
    runtime.free_mem -= size_of::<(ArrayObject)>() as i64 + data.as_ref().map(|x| x.len()).unwrap_or(0) as i64;
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
    runtime.free_mem -= size_of::<(ArrayObject)>() as i64 + data.as_ref().map(|x| x.len()).unwrap_or(0) as i64;
    let array_object = ArrayObject {
        is_null: data.is_none(),
        element_type_ref: None,
        element_type_str: String::from(element_type),
        elements: RefCell::new(data.unwrap_or(Vec::new())),
        code: runtime.get_next_object_code()
    };
    return Ok(Variable::ArrayReference(Rc::new(array_object)));
}

pub fn construct_null_object(_runtime: &mut Runtime, class: Rc<Class>) -> Result<Variable, RunnerError> {
    return Ok(Variable::Reference(class, None));
}

pub fn construct_null_object_by_name(runtime: &mut Runtime, name: &str) -> Result<Variable, RunnerError> {
    return parse_single_type_descriptor(runtime, name, true);
}

pub fn construct_object(runtime: &mut Runtime, name: &str) -> Result<Variable, RunnerError> {
    let debug = false;
    runnerPrint!(runtime, debug, 3, "Constructing object {}", name);
    try!(load_class(runtime, name));

    let original_class = try!(load_class(runtime, name));
    let total_size = original_class.total_size.borrow();
    let obj = Object::new(runtime, &original_class.clone(), *total_size);
    let mut class = original_class.clone();

    loop {
        runnerPrint!(runtime, debug, 3, "Constructing object of type {}", class.name);
        for field in &class.cr.fields {
            if field.access_flags & ACC_STATIC != 0 {
                continue;
            }

            let name_string = try!(class.cr.constant_pool.get_str(field.name_index));
            let descriptor_string = try!(class.cr.constant_pool.get_str(field.descriptor_index));

            let var = try!(initialise_variable(runtime, descriptor_string.as_str()));

            obj.put_member_at_offset(class.find_member_offset(&*name_string).unwrap(), var);
        }

        let maybe_super_class = class.super_class.borrow().clone();
        if maybe_super_class.is_some() {
            class = maybe_super_class.unwrap();
        } else {
            register_object(runtime, &obj);
            return Ok(Variable::Reference(original_class.clone(), Some(obj)));
        }
    }
}
