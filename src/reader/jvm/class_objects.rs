use reader::runner::*;
use reader::util::*;

pub fn get_primitive_class_object(runtime: &mut Runtime, descriptor: String) -> Result<Variable, RunnerError> {
    if descriptor.len() > 1 {
        panic!("Asked to make primitive class of type '{}'", descriptor);
    }

    {
        let maybe_existing = runtime.class_objects.get(&descriptor);
        if maybe_existing.is_some() {
            return Ok(maybe_existing.unwrap().clone());
        }
    }

    let var = try!(construct_object(runtime, &"java/lang/Class"));
    runtime.class_objects.insert(descriptor.clone(), var.clone());

    let name_object = try!(make_string(runtime, try!(descriptor_to_type_name(descriptor.as_str())).as_str()));
    let interned_string = try!(string_intern(runtime, &name_object));
    let statics = &var.to_ref().type_ref.statics;
    statics.borrow_mut().insert(String::from("initted"), Variable::Boolean(true));
    let members = &var.to_ref().members;
    members.borrow_mut().insert(String::from("name"), interned_string);
    members.borrow_mut().insert(String::from("__is_primitive"), Variable::Boolean(true));
    members.borrow_mut().insert(String::from("__is_array"), Variable::Boolean(false));

    return Ok(var);
}

pub fn get_class_object_from_descriptor(runtime: &mut Runtime, descriptor: &str) -> Result<Variable, RunnerError> {
    {
        let maybe_existing = runtime.class_objects.get(&String::from(descriptor));
        if maybe_existing.is_some() {
            return Ok(maybe_existing.unwrap().clone());
        }
    }

    let var = try!(construct_object(runtime, &"java/lang/Class"));
    runtime.class_objects.insert(String::from(descriptor), var.clone());

    let name_object = try!(make_string(runtime, try!(descriptor_to_type_name(descriptor)).as_str()));
    let interned_string = try!(string_intern(runtime, &name_object));
    try!(put_field(runtime, var.to_ref(), &"java/lang/Class", "name", interned_string));
    let statics = &var.to_ref().type_ref.statics;
    statics.borrow_mut().insert(String::from("initted"), Variable::Boolean(true));
    let members = &var.to_ref().members;

    let subtype = try!(parse_single_type_string(runtime, descriptor, false));
    let mut is_primitive = false;
    let mut is_array = false;
    let mut is_unresolved = false;
    match subtype {
        Variable::UnresolvedReference(ref _type_string) => {
            is_unresolved = true;
        },
        Variable::Reference(ref obj) => {
            let class = obj.type_ref.clone();
            members.borrow_mut().insert(String::from("__class"), try!(construct_null_object(runtime, class)));
        },
        Variable::ArrayReference(ref array_obj) => {
            is_array = true;
            let component_type;
            if array_obj.element_type_ref.is_some() {
                component_type = try!(get_class_object_from_descriptor(runtime, array_obj.element_type_str.clone().as_str()));
            } else {
                component_type = try!(get_primitive_class_object(runtime, array_obj.element_type_str.clone()));
            }
            members.borrow_mut().insert(String::from("__componentType"), component_type);
        },
        _ => { is_primitive = true; }
    }
    members.borrow_mut().insert(String::from("__is_primitive"), Variable::Boolean(is_primitive));
    members.borrow_mut().insert(String::from("__is_array"), Variable::Boolean(is_array));
    members.borrow_mut().insert(String::from("__is_unresolved"), Variable::Boolean(is_unresolved));

    return Ok(var);
}
