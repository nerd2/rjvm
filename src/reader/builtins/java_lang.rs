use reader::jvm::construction::*;
use reader::jvm::class_objects::*;
use reader::jvm::interpreter::*;
use reader::runner::*;
use reader::util::*;
use reader::class_reader::*;
use std;
use std::rc::Rc;

fn set_property(runtime: &mut Runtime, properties: &Variable, key: &str, value: &str) -> Result<(), RunnerError> {
    let keyvar = make_string(runtime, key).expect("Couldn't create string for argument");
    let valuevar = make_string(runtime, value).expect("Couldn't create string for argument");
    try!(invoke_nested(runtime, properties.to_ref_type().clone(), vec!(properties.clone(), keyvar, valuevar), "setProperty", "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/Object;", false));
    return Ok(());
}

fn make_field(runtime: &mut Runtime, clazz: &Variable, name: Rc<String>, descriptor: Rc<String>, access: u16, slot: i32)  -> Result<Variable, RunnerError> {
    let class_name = "java/lang/reflect/Field";
    let name_var = try!(make_string(runtime, name.as_str()));
    let name_var_interned = try!(string_intern(runtime, &name_var));
    let signature_var = try!(make_string(runtime, descriptor.as_str()));
    let var = try!(construct_object(runtime, class_name));
    try!(put_field(runtime, var.to_ref(), class_name, "name", name_var_interned));
    try!(put_field(runtime, var.to_ref(), class_name, "signature", signature_var));
    let type_obj = try!(get_class_object_from_descriptor(runtime, descriptor.as_str()));
    try!(put_field(runtime, var.to_ref(), class_name, "type", type_obj));
    try!(put_field(runtime, var.to_ref(), class_name, "slot", Variable::Int(slot)));
    try!(put_field(runtime, var.to_ref(), class_name, "clazz", clazz.clone()));
    try!(put_field(runtime, var.to_ref(), class_name, "modifiers", Variable::Int(access as i32)));
    return Ok(var);
}

fn make_method(runtime: &mut Runtime, name: Rc<String>, descriptor: Rc<String>, _access: u16)  -> Result<Variable, RunnerError> {
    let class_name = &"java/lang/reflect/Method";
    let name_var = try!(make_string(runtime, name.as_str()));
    let name_var_interned = try!(string_intern(runtime, &name_var));
    let signature_var = try!(make_string(runtime, descriptor.as_str()));
    let var = try!(construct_object(runtime, class_name));
    try!(put_field(runtime, var.to_ref(), class_name, "name", name_var_interned));
    try!(put_field(runtime, var.to_ref(), class_name, "signature", signature_var));
    return Ok(var);
}

pub fn try_builtin(class_name: &Rc<String>, method_name: &Rc<String>, descriptor: &Rc<String>, args: &Vec<Variable>, runtime: &mut Runtime) -> Result<bool, RunnerError> {
    match (class_name.as_str(), method_name.as_str(), descriptor.as_str()) {
        ("java/lang/Class", "registerNatives", "()V") => {}
        ("java/lang/Class", "isArray", "()Z") => {
            let obj = args[0].clone().to_ref();
            let members = obj.members.borrow();
            let value = members.get(&String::from("__is_array")).unwrap();
            runnerPrint!(runtime, true, 2, "BUILTIN: is_array {}", value);
            runtime.push_on_stack(value.clone());
        }
        ("java/lang/Class", "isPrimitive", "()Z") => {
            let obj = args[0].clone().to_ref();
            let members = obj.members.borrow();
            let value = members.get(&String::from("__is_primitive")).unwrap();
            runnerPrint!(runtime, true, 2, "BUILTIN: is_primitive {}", value);
            runtime.push_on_stack(value.clone());
        }
        ("java/lang/Class", "getPrimitiveClass", "(Ljava/lang/String;)Ljava/lang/Class;") => {
            let obj = args[0].clone().to_ref();
            let string = try!(extract_from_string(runtime, &obj));
            let descriptor = type_name_to_descriptor(&string);
            runnerPrint!(runtime, true, 2, "BUILTIN: getPrimitiveClass {} {}", string, descriptor);
            let var = try!(get_primitive_class_object(runtime, descriptor));
            runtime.push_on_stack(var);
        }
        ("java/lang/Class", "isAssignableFrom", "(Ljava/lang/Class;)Z") => {
            let class_object_1 = args[0].clone().to_ref();
            let mut class1 = class_object_1.members.borrow().get(&String::from("__class")).unwrap().to_ref_type();
            let class_object_2 = args[1].clone().to_ref();
            let class2 = class_object_2.members.borrow().get(&String::from("__class")).unwrap().to_ref_type();
            while class1 != class2 {
                if class1.super_class.borrow().is_none() { break; }
                let new_class1 = class1.super_class.borrow().clone().unwrap();
                class1 = new_class1;
            }

            runtime.push_on_stack(Variable::Boolean(class1 == class2));
        }
        ("java/lang/Class", "getComponentType", "()Ljava/lang/Class;") => {
            let class_object_1 = args[0].clone().to_ref();
            let is_array = class_object_1.members.borrow().get(&String::from("__is_array")).unwrap().to_bool();
            if !is_array {
                return Err(RunnerError::ClassInvalid2(format!("getComponentType on non-array {}", class_object_1)));
            }
            let var = class_object_1.members.borrow().get(&String::from("__componentType")).unwrap().clone();
            runnerPrint!(runtime, true, 2, "BUILTIN: getComponentType {}", var);

            runtime.push_on_stack(var);
        },
        ("java/lang/Class", "forName0", "(Ljava/lang/String;ZLjava/lang/ClassLoader;Ljava/lang/Class;)Ljava/lang/Class;") => {
            let descriptor_string_obj = args[0].clone().to_ref();
            let descriptor = try!(extract_from_string(runtime, &descriptor_string_obj));
            let initialize = args[1].to_bool();
            let ref class_loader = args[2];
            let ref caller_class = args[3];
            runnerPrint!(runtime, true, 2, "BUILTIN: forName0 {} {} {} {}", descriptor, initialize, class_loader, caller_class);
            // Load class
            let maybe_class = load_class(runtime, descriptor.as_str());
            if maybe_class.is_err() {
                let err = maybe_class.unwrap_err();
                match err {
                    RunnerError::ClassNotLoaded(ref _s) => {
                        let exception = try!(construct_object(runtime, &"java/lang/ClassNotFoundException"));
                        return Err(RunnerError::Exception(exception))
                    },
                    _ => return Err(err)
                };
            }

            let var = try!(get_class_object_from_descriptor(runtime, type_name_to_descriptor(&descriptor).as_str()));
            runtime.push_on_stack(var);
        }
        ("java/lang/Class", "desiredAssertionStatus0", "(Ljava/lang/Class;)Z") => {runtime.push_on_stack(Variable::Boolean(false));}
        ("java/lang/Class", "getDeclaredFields0", "(Z)[Ljava/lang/reflect/Field;") => {
            let class_obj = args[0].to_ref();
            let class = class_obj.members.borrow().get(&String::from("__class")).unwrap().to_ref_type();
            let public_only = args[1].to_bool();

            runnerPrint!(runtime, true, 2, "BUILTIN: getDeclaredFields0 {}", class.name);

            let mut field_objects : Vec<Variable> = Vec::new();
            let mut index = 0;
            for field in &class.cr.fields {
                if !public_only || (field.access_flags & ACC_PUBLIC != 0) {
                    let name_string = try!(class.cr.constant_pool.get_str(field.name_index));
                    let descriptor_string = try!(class.cr.constant_pool.get_str(field.descriptor_index));
                    let field_object = try!(make_field(runtime, &args[0], name_string, descriptor_string, field.access_flags, index));
                    field_objects.push(field_object);
                }

                index += 1;
            }
            let fields_array = try!(construct_array_by_name(runtime, &"java/lang/reflect/Field", Some(field_objects)));
            runtime.push_on_stack(fields_array);
        }
        ("java/lang/Class", "getDeclaredMethods0", "(Z)[Ljava/lang/reflect/Method;") => {
            let class_obj = args[0].to_ref();
            let class = class_obj.members.borrow().get(&String::from("__class")).unwrap().to_ref_type();
            let public_only = args[1].to_bool();

            let mut method_objects : Vec<Variable> = Vec::new();
            for method in &class.cr.methods {
                if public_only && (method.access_flags & ACC_PUBLIC == 0) {
                    continue;
                }

                let name_string = try!(class.cr.constant_pool.get_str(method.name_index));
                let descriptor_string = try!(class.cr.constant_pool.get_str(method.descriptor_index));
                let methods_object = try!(make_method(runtime, name_string, descriptor_string, method.access_flags));
                method_objects.push(methods_object);
            }
            let methods_array = try!(construct_array_by_name(runtime, &"java/lang/reflect/Method", Some(method_objects)));
            runtime.push_on_stack(methods_array);
        }
        ("java/lang/System", "arraycopy", "(Ljava/lang/Object;ILjava/lang/Object;II)V") => {
            runnerPrint!(runtime, true, 2, "BUILTIN: arrayCopy {} {} {} {} {}", args[0], args[1], args[2], args[3], args[4]);

            let src = args[0].to_arrayobj();
            let src_pos = args[1].to_int();
            let dest = args[2].to_arrayobj();
            let dest_pos = args[3].to_int();
            let length = args[4].to_int();

            if src.is_null || dest.is_null {
                let exception = try!(construct_object(runtime, &"java/lang/NullPointerException"));
                return Err(RunnerError::Exception(exception));
            }

            let src_data = src.elements.borrow();
            let mut dest_data = dest.elements.borrow_mut();

            for n in 0..length {
                dest_data[(dest_pos + n) as usize] = src_data[(src_pos + n) as usize].clone();
            }
        },
        ("java/lang/System", "registerNatives", "()V") => {
        },
        ("java/lang/System", "initProperties", "(Ljava/util/Properties;)Ljava/util/Properties;") => {
            let properties = args[0].clone();
            try!(invoke_nested(runtime, properties.to_ref_type().clone(), vec!(properties.clone()), "<init>", "()V", false));
            runnerPrint!(runtime, true, 2, "BUILTIN: initProperties {}", properties);
            try!(set_property(runtime, &properties, "file.encoding", "us-ascii"));
            runtime.push_on_stack(properties);
        },
        ("java/lang/System", "setIn0", "(Ljava/io/InputStream;)V") => {
            let stream = args[0].clone();
            runnerPrint!(runtime, true, 2, "BUILTIN: setIn0 {}", stream);
            try!(put_static(runtime, "java/lang/System", "in", stream));
        },
        ("java/lang/System", "setOut0", "(Ljava/io/PrintStream;)V") => {
            let stream = args[0].clone();
            runnerPrint!(runtime, true, 2, "BUILTIN: setOut0 {}", stream);
            try!(put_static(runtime, "java/lang/System", "out", stream));
        },
        ("java/lang/System", "setErr0", "(Ljava/io/PrintStream;)V") => {
            let stream = args[0].clone();
            runnerPrint!(runtime, true, 2, "BUILTIN: setErr0 {}", stream);
            try!(put_static(runtime, "java/lang/System", "err", stream));
        },
        ("java/lang/System", "loadLibrary", "(Ljava/lang/String;)V") => {
            let lib_string_obj = args[0].clone().to_ref();
            let lib = try!(extract_from_string(runtime, &lib_string_obj));
            runnerPrint!(runtime, true, 2, "BUILTIN: loadLibrary {}", lib);
        }
        ("java/lang/Runtime", "availableProcessors", "()I") => {
            runnerPrint!(runtime, true, 2, "BUILTIN: availableProcessors");
            runtime.push_on_stack(Variable::Int(1));
        },
        ("java/lang/Object", "registerNatives", "()V") => {return Ok(true)},
        ("java/lang/Object", "notifyAll", "()V") => {
            runnerPrint!(runtime, true, 2, "BUILTIN: TODO notifyAll {}", args[0]);
            return Ok(true)
        },
        ("java/lang/String", "intern", "()Ljava/lang/String;") => {
            let interned = try!(string_intern(runtime, &args[0]));
            runnerPrint!(runtime, true, 2, "BUILTIN: intern {} {:p}", args[0], &*interned.to_ref());
            runtime.push_on_stack(interned);
        },
        ("java/lang/Float", "floatToRawIntBits", "(F)I") => {
            let float = args[0].to_float();
            let bits = unsafe {std::mem::transmute::<f32, u32>(float)};
            runnerPrint!(runtime, true, 2, "BUILTIN: floatToRawIntBits {} {}", float, bits);
            runtime.push_on_stack(Variable::Int(bits as i32));
        },
        ("java/lang/Float", "intBitsToFloat", "(I)F") => {
            let int = args[0].to_int();
            let float = unsafe {std::mem::transmute::<i32, f32>(int)};
            runnerPrint!(runtime, true, 2, "BUILTIN: intBitsToFloat {} {}", int, float);
            runtime.push_on_stack(Variable::Float(float));
        },
        ("java/lang/Double", "doubleToRawLongBits", "(D)J") => {
            let double = args[0].to_double();
            let bits = unsafe {std::mem::transmute::<f64, u64>(double)};
            runnerPrint!(runtime, true, 2, "BUILTIN: doubleToRawIntBits {} {}", double, bits);
            runtime.push_on_stack(Variable::Long(bits as i64));
        },
        ("java/lang/Double", "longBitsToDouble", "(J)D") => {
            let long = args[0].to_long();
            let double = unsafe {std::mem::transmute::<i64, f64>(long)};
            runnerPrint!(runtime, true, 2, "BUILTIN: doubleToRawIntBits {} {}", long, double);
            runtime.push_on_stack(Variable::Double(double));
        },
        ("java/lang/SecurityManager", "checkPermission", "(Ljava/security/Permission;)V") => {
        },
        ("java/lang/Object", "hashCode", "()I") => {
            let code = try!(args[0].hash_code(runtime));
            runnerPrint!(runtime, true, 2, "BUILTIN: hashcode {}", code);
            runtime.push_on_stack(Variable::Int(code));
        },
        ("java/lang/System", "identityHashCode", "(Ljava/lang/Object;)I") => {
            let code = try!(args[0].hash_code(runtime));
            runnerPrint!(runtime, true, 2, "BUILTIN: identityHashCode {}", code); // TODO test
            runtime.push_on_stack(Variable::Int(code));
        },
        ("java/lang/Object", "getClass", "()Ljava/lang/Class;") => {
            let ref descriptor = args[0].get_descriptor();
            let var = try!(get_class_object_from_descriptor(runtime, descriptor.as_str()));
            runnerPrint!(runtime, true, 2, "BUILTIN: getClass {} {}", descriptor, var);
            runtime.push_on_stack(var);
        },
        ("java/lang/ClassLoader", "registerNatives", "()V") => {},
        ("java/lang/Thread", "registerNatives", "()V") => {},
        ("java/lang/Thread", "isAlive", "()Z") => {
            let obj = args[0].clone().to_ref();
            let members = obj.members.borrow();
            let var = members.get(&String::from("__alive")).unwrap_or(&Variable::Boolean(false)).clone();
            runnerPrint!(runtime, true, 2, "BUILTIN: isAlive {}", var);
            runtime.push_on_stack(var);
        },
        ("java/lang/Thread", "start0", "()V") => {
            // TODO
        }
        ("java/lang/Thread", "setPriority0", "(I)V") => {
            let obj = args[0].clone().to_ref();
            runnerPrint!(runtime, true, 2, "BUILTIN: setPriority0 {} {}", args[0], args[1]);
            try!(put_field(runtime, obj.clone(), &"java/lang/Thread", &"priority", args[1].clone()));
        }
        ("java/lang/Thread", "currentThread", "()Ljava/lang/Thread;") => {
            runnerPrint!(runtime, true, 2, "BUILTIN: currentThread");
            if runtime.current_thread.is_none() {
                runnerPrint!(runtime, true, 2, "BUILTIN: currentThread - creating thread");
                let thread_group;
                {
                    let var = try!(construct_object(runtime, &"java/lang/ThreadGroup"));
                    let obj = var.to_ref();
                    try!(invoke_nested(runtime, obj.type_ref.clone(), vec!(var.clone()), "<init>", "()V", false));
                    thread_group = var.clone();
                }

                {
                    let var = try!(construct_object(runtime, &"java/lang/Thread"));

                    runtime.current_thread = Some(var.clone());
                    let obj = var.to_ref();
                    let mut members = obj.members.borrow_mut();
                    members.insert(String::from("name"), try!(make_string(runtime, &"thread")));
                    members.insert(String::from("priority"), Variable::Int(1));
                    members.insert(String::from("group"), thread_group);
                    members.insert(String::from("__alive"), Variable::Boolean(true));
                }
            }
            let thread = runtime.current_thread.as_ref().unwrap().clone();
            runtime.push_on_stack(thread);
        },
        _ => return Ok(false)
    };
    return Ok(true);
}
