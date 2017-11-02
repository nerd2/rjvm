use reader::runner::*;
use std;
use std::rc::Rc;

pub fn make_string(runtime: &mut Runtime, val: &str) -> Result<Variable, RunnerError> {
    let var = try!(construct_object(runtime, &"java/lang/String"));
    let obj = var.to_ref();
    let array = construct_char_array(runtime,val);
    try!(put_field(runtime, obj, &"java/lang/String", &"value", array));
    return Ok(var);
}

pub fn string_intern(runtime: &mut Runtime, var: &Variable) -> Result<Variable, RunnerError> {
    let obj = var.to_ref();
    let string = try!(extract_from_string(runtime, &obj));
    if !runtime.string_interns.contains_key(&string) {
        runtime.string_interns.insert(string.clone(), var.clone());
    }
    return Ok(runtime.string_interns.get(&string).unwrap().clone());
}

pub fn extract_from_char_array(runtime: &mut Runtime, var: &Variable) -> Result<String, RunnerError> {
    let array = var.to_arrayobj();
    if array.is_null {
        let exception = try!(construct_object(runtime, &"java/lang/NullPointerException"));
        return Err(RunnerError::Exception(exception));
    } else {
        let mut res = String::new();
        for c in array.elements.borrow().iter() {
            res.push(c.to_char());
        }
        return Ok(res);
    }
}

pub fn extract_from_string(runtime: &mut Runtime, obj: &Rc<Object>) -> Result<String, RunnerError> {
    let field = try!(get_field(runtime, obj, "java/lang/String", "value"));
    let string = try!(extract_from_char_array(runtime, &field));
    return Ok(string);
}

pub fn string_to_string(obj: &Object) -> String {
    let members = obj.members.borrow();
    let value_array = members.get(&String::from("value"));
    if value_array.is_none() { return String::from("");}
    let array = value_array.unwrap().to_arrayobj();
    if array.is_null { return String::from("");}
    let vec = array.elements.borrow();
    let mut ret = String::new();
    for v in vec.iter() {
        ret.push(v.to_char());
    }

    return ret;
}

pub fn type_name_to_descriptor(name: &String) -> String {
    return String::from(match name.as_str() {
        "byte" => "B",
        "char" => "C",
        "double" => "D",
        "float" => "F",
        "int" => "I",
        "long" => "J",
        "short" => "S",
        "boolean" => "Z",
        _ => {
            let mut ret = String::from("L");
            ret.push_str(name.as_str());
            ret.push(';');
            return ret;
        }
    });
}

pub fn rc_ptr_eq<T: ?Sized>(this: &Rc<T>, other: &Rc<T>) -> bool
    where T: std::fmt::Display
{
    let this_ptr: *const T = &**this;
    let other_ptr: *const T = &**other;
    debugPrint!(false, 2, "RC ptr eq {} {:p} {} {:p}", this, this_ptr, other, other_ptr);
    this_ptr == other_ptr
}

pub fn generate_class_descriptor(class: &Rc<Class>) -> String {
    let mut ret = String::new();
    ret.push('L');
    ret.push_str(class.name.as_str());
    ret.push(';');
    return ret;
}

pub fn generate_method_descriptor(args: &Vec<Variable>, return_descriptor: String, is_static: bool) -> String {
    let mut ret = String::new();
    ret.push('(');
    for arg in args.iter().skip(if is_static {0} else {1}) {
        ret.push_str(arg.get_descriptor().as_str());
    }
    ret.push(')');
    ret.push_str(return_descriptor.as_str());
    return ret;
}
