extern crate rand;
extern crate zip;
use reader::class_reader::*;
use reader::jvm::construction::*;
use reader::jvm::interpreter::*;
pub use reader::types::class::*;
pub use reader::types::frame::*;
pub use reader::types::objects::*;
pub use reader::types::runtime::*;
pub use reader::types::variable::*;
pub use reader::util::make_string;
use reader::util::*;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::io::Read;
use std::io::Cursor;
use std::rc::Rc;
use std::path::PathBuf;

lazy_static! {
    static ref builtin_class_fields: HashMap<&'static str, Vec<&'static str>> = {
        let mut m = HashMap::new();
        m.insert("java/lang/Class", vec!("__is_array", "__is_primitive", "__class", "__componentType", "__is_unresolved"));
        m.insert("java/lang/Thread", vec!("__alive"));
        m
    };
}

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
                        loop {
                            for e in &runtime.current_frame.code.exceptions.clone() {
                                if current_position >= e.start_pc as u64 && current_position <= e.end_pc as u64 {
                                    if e.catch_type > 0 {
                                        let class_name = try!(runtime.current_frame.constant_pool.get_class_name(e.catch_type));
                                        if exception.to_ref().unwrap().type_ref().name != *class_name {
                                            continue;
                                        }
                                    }

                                    runnerPrint!(runtime, true, 3, "Caught exception and branching to {}", e.handler_pc);

                                    caught = true;
                                    runtime.push_on_stack(exception.clone());
                                    runtime.current_frame.return_pos = e.handler_pc as u64;
                                    break;
                                }
                            }

                            if caught == true || runtime.previous_frames.len() == 0 {
                                break;
                            }

                            runtime.current_frame = runtime.previous_frames.pop().unwrap();
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
                    runnerPrint!(runtime, true, 3, "Uncaught");
                    return Err(err);
                } else {
                    break;
                }
            }
        }
    }
}

fn do_find_class<T : Read>(debug: bool, name: &str, reader: T) -> Option<ClassResult> {
    let mut buf_reader = BufReader::new(reader);

    let maybe_class_result = read_stage_1(&mut buf_reader);
    if maybe_class_result.is_err() {
        debugPrint!(debug, 3, "Couldn't read headers of class file {:?}", maybe_class_result.unwrap_err());
        return None;
    }

    let mut class_result = maybe_class_result.unwrap();

    if class_result.name().map(|x| *x != name).unwrap_or(true) {
        debugPrint!(debug, 3, "Name mismatch {:?}, {}", class_result.name(), name);
        return None;
    }

    let maybe_read = read_stage_2(&mut buf_reader, &mut class_result);

    if maybe_read.is_err() {
        debugPrint!(debug, 3, "Failed to read rest of file {:?}", maybe_read.unwrap_err());
        return None;
    }

    return Some(class_result);
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
        let maybe_file = File::open(direct_path.as_path());
        if maybe_file.is_err() {
            runnerPrint!(runtime, debug, 3, "Couldn't open class file {}", maybe_file.unwrap_err());
            continue;
        }

        let maybe_class = do_find_class(debug, name.as_str(), maybe_file.unwrap());

        if maybe_class.is_some() {
            return Ok(maybe_class.unwrap());
        }
    }

    for jar in runtime.jars.iter_mut() {
        let maybe_zip_file = jar.by_name((name.clone() + ".class").as_str());
        if maybe_zip_file.is_err() {
            runnerPrint!(runtime, debug, 3, "Couldn't find class {} in jar", name.as_str());
            continue;
        }

        let maybe_class = do_find_class(debug, name.as_str(), maybe_zip_file.unwrap());

        if maybe_class.is_some() {
            return Ok(maybe_class.unwrap());
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
    let debug = true;

    let core_class = Rc::new(Class::new(&String::from(name), class_result));
    let mut class_chain : Vec<Rc<Class>> = Vec::new();
    let mut member_count : usize = 0;

    runtime.classes.insert(String::from(name), core_class.clone());
    runnerPrint!(runtime, debug, 1, "Bootstrapping {}", name);

    // Loop down superclass chain
    let mut class = core_class.clone();
    while !*class.initialising.borrow() && !*class.initialised.borrow() {
        class_chain.push(class.clone());

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
                member_count = *maybe_superclass.as_ref().unwrap().total_size.borrow();
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

    for class in class_chain.iter().rev() {
        for field in class.cr.fields.iter() {
            let name_string = try!(class.cr.constant_pool.get_str(field.name_index));
            let descriptor_string = try!(class.cr.constant_pool.get_str(field.descriptor_index));

            if field.access_flags & ACC_STATIC == 0 {
                class.set_member_offset(name_string, member_count);
                member_count = member_count + 1;
            } else {
                runnerPrint!(runtime, debug, 3, "Constructing class static member {} {}", name_string, descriptor_string);

                let var = try!(initialise_variable(runtime, descriptor_string.as_str()));

                runnerPrint!(runtime, debug, 3, "Constructed with {}", var);

                class.statics.borrow_mut().insert((*name_string).clone(), var);
            }
        }

        builtin_class_fields.get(class.name.as_str()).map(|extra_fields|
            for field in extra_fields {
                class.set_member_offset(Rc::new(String::from(*field)), member_count);
                member_count = member_count + 1;
            });
        *class.total_size.borrow_mut() = member_count;
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
    let mut class : Option<Rc<Class>> = None;
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
                class = Some(try!(load_class(runtime, type_string.as_str())));
                variable = try!(construct_null_object(runtime, class.clone().unwrap()));
            } else {
                if runtime.classes.contains_key(type_string.as_str()) {
                    class = Some(runtime.classes.get(type_string.as_str()).unwrap().clone());
                    variable = try!(construct_null_object(runtime, class.clone().unwrap()));
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
            return Ok(try!(construct_array(runtime, class.unwrap(), None)));
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

fn execute_method(runtime: &mut Runtime, class: &str, method: &str, descriptor: &str, _args: &Vec<Variable>, ret: bool) -> Result<Variable, RunnerError> {
    //runtime.add_arguments(arguments);
    let _ignore = runtime.invoke(Rc::new(String::from(class)), Rc::new(String::from(method)), Rc::new(String::from(descriptor)), false, false);
    try!(do_run_method(runtime));
    if ret {
        return Ok(runtime.pop_from_stack().unwrap().clone());
    } else {
        return Ok(Variable::Int(0));
    }
}

pub fn run(class_paths: &Vec<String>, jars: Vec<zip::ZipArchive<File>>, class: &ClassResult) -> Result<(), RunnerError> {
    println!("Running");
    let mut runtime = Runtime::new(class_paths.clone(), jars);
    runtime.current_frame.constant_pool = class.constant_pool.clone();

    try!(bootstrap_class_and_dependencies(&mut runtime, String::new().as_str(), class));

    let main_code = try!(class.get_code(&"main", &"([Ljava/lang/String;)V"));
    runtime.current_frame.code = main_code;

    try!(do_run_method(&mut runtime));

    return Ok(());
}

pub fn get_runtime(class_paths: &Vec<String>, jars: Vec<zip::ZipArchive<File>>, initialise: bool) -> Runtime {
    let mut runtime = Runtime::new(class_paths.clone(), jars);

    if initialise {
        let execution_result = execute_method(&mut runtime, "java/lang/System", "initializeSystemClass", "()V", &Vec::new(), false);
        execution_result.expect("Failed to initialize system");
    }

    return runtime;
}

pub fn run_method(runtime: &mut Runtime, class_result: &ClassResult, method: &str, arguments: &Vec<Variable>, return_descriptor: String) -> Result<Variable, RunnerError> {
    println!("Running method {} with {} arguments", method, arguments.len());

    runtime.reset_frames();
    runtime.current_frame.constant_pool = class_result.constant_pool.clone();

    let name = try!(class_result.name());
    let class = try!(bootstrap_class_and_dependencies(runtime, name.as_str(), class_result));

    runtime.current_frame.class = Some(class);
    runtime.add_arguments(arguments);

    let method_descriptor = generate_method_descriptor(&arguments, return_descriptor.clone(), true);
    runnerPrint!(runtime, true, 1, "Finding method {} with descriptor {}", method, method_descriptor);
    let code = try!(class_result.get_code(method, method_descriptor.as_str()));

    println!("Running method");
    runtime.current_frame.code = code;
    try!(do_run_method(runtime));

    if return_descriptor == "V" {
        return Ok(Variable::Int(0));
    } else {
        return Ok(runtime.pop_from_stack().unwrap().clone());
    }
}
