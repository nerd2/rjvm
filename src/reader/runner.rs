
extern crate rand;
use reader::class_reader::*;
use reader::jvm::construction::*;
use reader::jvm::interpreter::*;
pub use reader::types::class::*;
pub use reader::types::frame::*;
pub use reader::types::objects::*;
pub use reader::types::runtime::*;
pub use reader::types::variable::*;
use reader::util::*;
use std::io;
use std::io::Cursor;
use std::rc::Rc;
use std::path::Path;
use std::path::PathBuf;
use glob::glob;


macro_rules! runnerPrint {
    ($runtime:expr, $enabled:expr, $level:expr, $fmt:expr) => {{if $enabled && $level <= PRINT_LEVEL!() { for _ in 1..$runtime.previous_frames.len() {print!("|"); } print!("{}: ", $runtime.count); println!($fmt); } }};
    ($runtime:expr, $enabled:expr, $level:expr, $fmt:expr, $($arg:tt)*) => {{if $enabled && $level <= PRINT_LEVEL!() { for _ in 1..$runtime.previous_frames.len() {print!("|"); } print!("{}: ", $runtime.count); println!($fmt, $($arg)*); } }};
}

#[derive(Debug)]
pub enum RunnerError {
    ClassInvalid(&'static str),
    ClassInvalid2(String),
    InvalidPc,
    IoError,
    UnknownOpCode(u8),
    ClassNotLoaded(String),
    Exception(Variable),
    Return,
    Invoke
}

impl From<io::Error> for RunnerError {
    fn from(_err: io::Error) -> RunnerError {
        RunnerError::IoError
    }
}
impl From<ClassReadError> for RunnerError {
    fn from(err: ClassReadError) -> RunnerError {
        RunnerError::ClassInvalid2(format!("{:?}", err))
    }
}


pub fn do_run_method(runtime: &mut Runtime) -> Result<(), RunnerError> {
    let start_frames = runtime.previous_frames.len();

    loop {
        let code = runtime.current_frame.code.clone();
        let mut buf = Cursor::new(&code.code);
        let name_string = runtime.current_frame.name.clone();

        buf.set_position(runtime.current_frame.return_pos);

        loop {
            let current_position = buf.position();
            let result = step(runtime, name_string.as_str(), &mut buf);
            if result.is_err() {
                let mut caught = false;
                let err = result.err().unwrap();
                match &err {
                    &RunnerError::Exception(ref exception) => {
                        runnerPrint!(runtime, true, 3, "Exception {}", exception);
                        for e in &runtime.current_frame.code.exceptions.clone() {
                            if current_position >= e.start_pc as u64 && current_position <= e.end_pc as u64 {
                                if e.catch_type > 0 {
                                    let class_name = try!(runtime.current_frame.constant_pool.get_class_name(e.catch_type));
                                    if exception.to_ref().type_ref.name != *class_name {
                                        continue;
                                    }
                                }

                                runnerPrint!(runtime, true, 3, "Caught exception and branching to {}", e.handler_pc);

                                caught = true;
                                runtime.push_on_stack(exception.clone());
                                buf.set_position(e.handler_pc as u64);
                                break;
                            }
                        }
                    },
                    &RunnerError::Invoke => {
                        let len = runtime.previous_frames.len();
                        runtime.previous_frames[len - 1].return_pos = buf.position();
                        break;
                    },
                    &RunnerError::Return => {
                        if runtime.previous_frames.len() < start_frames {
                            return Ok(());
                        }
                        break;
                    }
                    _ => {}
                }

                if caught == false {
                    return Err(err);
                }
            }
        }
    }
}

fn find_class(runtime: &mut Runtime, base_name: &str) -> Result<ClassResult, RunnerError> {
    let debug = false;
    let mut name = String::from(base_name);
    name = name.replace('.', "/");
    runnerPrint!(runtime, debug, 3, "Finding class {}", name);
    for class_path in runtime.class_paths.iter() {
        let mut direct_path = PathBuf::from(class_path);
        for sub in name.split('/') {
            direct_path.push(sub)
        }
        direct_path.set_extension("class");
        runnerPrint!(runtime, debug, 3, "Trying path {}", direct_path.display());
        let direct_classname = get_classname(direct_path.as_path());
        if direct_classname.is_ok() && *direct_classname.as_ref().unwrap() == name {
            runnerPrint!(runtime, debug, 3, "Name matched for {}", name);
            let maybe_read = read(Path::new(&direct_path));
            if maybe_read.is_ok() {
                return Ok(maybe_read.unwrap());
            }
        }

        if false {
            runnerPrint!(runtime, debug, 3, "Finding class {} direct load failed ({}), searching {}",
                name, match &direct_classname {
                    &Ok(ref x) => x.clone(),
                    &Err(ref y) => format!("{:?}", y),
                }, class_path);

            // Else try globbing
            let mut glob_path = class_path.clone();
            glob_path.push_str("/**/*.class");
            let maybe_glob = glob(glob_path.as_str());
            if maybe_glob.is_err() {
                runnerPrint!(runtime, true, 1, "Error globbing class path {}", class_path);
                continue;
            }

            let class_match = maybe_glob.unwrap()
                .filter_map(Result::ok)
                .filter(|x| {
                    let classname = get_classname(&x);
                    return classname.is_ok() && classname.unwrap() == name;
                })
                .nth(0);

            if class_match.is_none() {
                runnerPrint!(runtime, debug, 2, "Could not find {} on class path {}", name, class_path);
                continue;
            }

            let maybe_read = read(&class_match.unwrap());
            if maybe_read.is_err() {
                runnerPrint!(runtime, true, 1, "Error reading class {} on class path {}", name, class_path);
                continue;
            }

            return Ok(maybe_read.unwrap());
        } else {
            runnerPrint!(runtime, debug, 2, "Could not find {} on class path {} (Error {:?})", name, class_path, direct_classname);
            continue;
        }
    }
    return Err(RunnerError::ClassNotLoaded(String::from(name)));
}

pub fn load_class(runtime: &mut Runtime, name: &str) -> Result<Rc<Class>, RunnerError> {
    {
        let maybe_class = runtime.classes.get(name).map(|x| x.clone());
        if maybe_class.is_some() {
            let x = maybe_class.unwrap().clone();
            try!(Class::initialise(runtime, &x));
            return Ok(x);
        }
    }
    runnerPrint!(runtime, true, 2, "Finding class {} not already loaded", name);
    let class_result = try!(find_class(runtime,name));
    let class_obj = try!(bootstrap_class_and_dependencies(runtime, name, &class_result));

    return Ok(class_obj);
}

fn bootstrap_class_and_dependencies(runtime: &mut Runtime, name: &str, class_result: &ClassResult) -> Result<Rc<Class>, RunnerError>  {
    let debug = false;

    let core_class = Rc::new(Class::new(&String::from(name), class_result));
    runtime.classes.insert(String::from(name), core_class.clone());
    runnerPrint!(runtime, debug, 1, "Bootstrapping {}", name);

    // Loop down superclass chain
    let mut class = core_class.clone();
    while !*class.initialising.borrow() && !*class.initialised.borrow() {
        // Initialise variables, refs can be unresolved
        for field in &class.cr.fields {
            if field.access_flags & ACC_STATIC == 0 {
                continue;
            }

            let name_string = try!(class.cr.constant_pool.get_str(field.name_index));
            let descriptor_string = try!(class.cr.constant_pool.get_str(field.descriptor_index));

            runnerPrint!(runtime, debug, 3, "Constructing class static member {} {}", name_string, descriptor_string);

            let var = try!(initialise_variable(runtime, descriptor_string.as_str()));

            runnerPrint!(runtime, debug, 3, "Constructed with {}", var);

            class.statics.borrow_mut().insert((*name_string).clone(), var);
        }

        let super_class_name =
            if class.cr.super_class_index > 0 {
                (*try!(class.cr.constant_pool.get_class_name(class.cr.super_class_index))).clone()
            } else if class.name != "java/lang/Object" {
                String::from("java/lang/Object")
            } else {
                break;
            };

        runnerPrint!(runtime, debug, 3, "Class {} has superclass {}", class.name, super_class_name);
        {
            let maybe_superclass = runtime.classes.get(&super_class_name);
            if maybe_superclass.is_some() {
                *class.super_class.borrow_mut() = Some(maybe_superclass.unwrap().clone());
                break;
            }
        }

        runnerPrint!(runtime, debug, 2, "Finding super class {} not already loaded", super_class_name);
        let class_result = try!(find_class(runtime, super_class_name.as_str()));
        let new_class = Rc::new(Class::new(&super_class_name, &class_result));
        runtime.classes.insert(super_class_name, new_class.clone());
        *class.super_class.borrow_mut() = Some(new_class.clone());

        class = new_class;
    }
    try!(Class::initialise(runtime, &core_class));
    runnerPrint!(runtime, debug, 1, "Bootstrap totally complete on {}", name);
    return Ok(core_class);
}

pub fn parse_single_type_descriptor(runtime: &mut Runtime, descriptor: &str, resolve: bool) -> Result<Variable, RunnerError> {
    let mut iter = descriptor.chars();

    let mut maybe_type_specifier = iter.next();

    if maybe_type_specifier.is_none() {
        return Err(RunnerError::ClassInvalid("Type specifier blank"));
    }

    let mut array_depth = 0;
    while maybe_type_specifier.map(|x| x=='[').unwrap_or(false) {
        array_depth = array_depth + 1;
        maybe_type_specifier = iter.next();
    }

    if maybe_type_specifier.is_none() {
        return Err(RunnerError::ClassInvalid2(format!("Type specifier invalid {}", descriptor)));
    }

    let variable;
    match maybe_type_specifier.unwrap() {
        'B' => variable = Variable::Byte(0),
        'C' => variable = Variable::Char('\0'),
        'D' => variable = Variable::Double(0.0),
        'F' => variable = Variable::Float(0.0),
        'I' => variable = Variable::Int(0),
        'J' => variable = Variable::Long(0),
        'S' => variable = Variable::Short(0),
        'Z' => variable = Variable::Boolean(false),
        _ => {
            let type_string : String =
                if maybe_type_specifier.unwrap() == 'L' {
                    iter.take_while(|x| *x != ';').collect()
                } else {
                    String::from(descriptor)
                };
            if resolve {
                let class = try!(load_class(runtime, type_string.as_str()));
                variable = try!(construct_null_object(runtime, class));
            } else {
                if runtime.classes.contains_key(type_string.as_str()) {
                    let class = runtime.classes.get(type_string.as_str()).unwrap().clone();
                    variable = try!(construct_null_object(runtime, class));
                } else {
                    variable = Variable::UnresolvedReference(type_string.clone());
                }
            }
        }
    }

    if array_depth > 0 {
        if array_depth > 1 {
            runnerPrint!(runtime, true, 1, "Warning: >1 array depth, is this right?");
        }
        if variable.is_primitive() {
            return Ok(try!(construct_primitive_array(runtime, variable.get_descriptor().as_str(), None)));
        } else if variable.is_unresolved() {
            return Ok(Variable::UnresolvedReference(String::from(descriptor)));
        } else {
            return Ok(try!(construct_array(runtime, variable.to_ref().type_ref.clone(), None)));
        }
    } else {
        return Ok(variable);
    }
}

pub fn parse_function_type_descriptor(runtime: &mut Runtime, descriptor: &str) -> Result<(Vec<Variable>, Option<Variable>), RunnerError> {
    let debug = false;
    let mut iter = descriptor.chars().peekable();

    if iter.next().map(|x| x!='(').unwrap_or(true) {
        runnerPrint!(runtime, true, 2, "Function type '{}' invalid", descriptor);
        return Err(RunnerError::ClassInvalid2(format!("Function type {} invalid", descriptor)));
    }

    let mut parameters : Vec<Variable> = Vec::new();
    let mut type_char : char;
    while {type_char = try!(iter.next().ok_or(RunnerError::ClassInvalid2(format!("Failed to parse {}", descriptor)))); type_char != ')'} {
        let mut type_string = String::new();
        while type_char == '[' {
            type_string.push(type_char);
            type_char = try!(iter.next().ok_or(RunnerError::ClassInvalid2(format!("Failed to parse {}", descriptor))));
        }
        type_string.push(type_char);

        if type_char == 'L' {
            type_string.push_str(iter.by_ref().take_while(|x| *x != ';').collect::<String>().as_str());
        }
        runnerPrint!(runtime, debug, 3, "Found parameter {}", type_string);
        let param = try!(parse_single_type_descriptor(runtime, type_string.as_str(), true));
        if !param.is_type_1() {
            parameters.push(param.clone());
        }
        parameters.push(param);
        runnerPrint!(runtime, debug, 3, "Parameters now {:?}", parameters);
    }

    let return_type_string : String = iter.collect();
    runnerPrint!(runtime, debug, 3, "Return type {}", return_type_string);
    if return_type_string == "V" {
        return Ok((parameters, None));
    } else {
        return Ok((parameters, Some(try!(parse_single_type_descriptor(runtime, return_type_string.as_str(), true)))));
    }
}

pub fn run(class_paths: &Vec<String>, class: &ClassResult) -> Result<(), RunnerError> {
    println!("Running");
    let mut runtime = Runtime::new(class_paths.clone());
    runtime.current_frame.constant_pool = class.constant_pool.clone();

    try!(bootstrap_class_and_dependencies(&mut runtime, String::new().as_str(), class));

    let main_code = try!(class.get_code(&"main", &"([Ljava/lang/String;)V"));
    runtime.current_frame.code = main_code;

    try!(do_run_method(&mut runtime));

    return Ok(());
}

pub fn get_runtime(class_paths: &Vec<String>) -> Runtime {
    return Runtime::new(class_paths.clone());
}

pub fn run_method(runtime: &mut Runtime, class_result: &ClassResult, method: &str, arguments: &Vec<Variable>, return_descriptor: String) -> Result<Variable, RunnerError> {
    println!("Running method {} with {} arguments", method, arguments.len());

    runtime.reset_frames();
    runtime.current_frame.constant_pool = class_result.constant_pool.clone();

    let name = try!(class_result.name());
    let class = try!(bootstrap_class_and_dependencies(runtime, name.as_str(), class_result));

    runtime.current_frame.class = Some(class);
    runtime.add_arguments(arguments);

    let method_descriptor = generate_method_descriptor(&arguments, return_descriptor, true);
    runnerPrint!(runtime, true, 1, "Finding method {} with descriptor {}", method, method_descriptor);
    let code = try!(class_result.get_code(method, method_descriptor.as_str()));

    println!("Running method");
    runtime.current_frame.code = code;
    try!(do_run_method(runtime));

    return Ok(runtime.pop_from_stack().unwrap().clone());
}