use reader::jvm::construction::*;
use reader::jvm::interpreter::*;
use reader::runner::*;
use std::rc::Rc;

pub fn try_builtin(class_name: &Rc<String>, method_name: &Rc<String>, descriptor: &Rc<String>, args: &Vec<Variable>, runtime: &mut Runtime) -> Result<bool, RunnerError> {
    match (class_name.as_str(), method_name.as_str(), descriptor.as_str()) {
        ("java/net/InetAddress", "init", "()V") => {}
        ("java/net/InetAddressImplFactory", "isIPv6Supported", "()Z") => { runtime.push_on_stack(Variable::Boolean(false)); }
        ("java/util/concurrent/atomic/AtomicLong", "VMSupportsCS8", "()Z") => { runtime.push_on_stack(Variable::Boolean(false)); }
        ("java/io/FileInputStream", "initIDs", "()V") => {},
        ("java/io/FileOutputStream", "initIDs", "()V") => {},
        ("java/io/FileOutputStream", "writeBytes", "([BIIZ)V") => {
            let fos = args[0].clone().to_ref();
            let bytes = args[1].clone().to_arrayobj();
            let offset = args[2].to_int();
            let length = args[3].to_int();
            let _append = args[4].to_bool();

            let file_descriptor = try!(get_field(runtime, &fos, &"java/io/FileOutputStream", "fd")).to_ref();
            let file_descriptor_id = try!(get_field(runtime, &file_descriptor, &"java/io/FileDescriptor", "fd")).to_int();

            let stream = match file_descriptor_id {
                1 => &mut runtime.stdout,
                2 => &mut runtime.stderr,
                _ => return Ok(true)
            };

            let data = bytes.elements.borrow();
            for n in offset..offset+length {
                stream.push(data[n as usize].to_byte() as char);
            }
        },
        ("java/io/FileDescriptor", "initIDs", "()V") => {},
        ("java/security/AccessController", "doPrivileged", "(Ljava/security/PrivilegedAction;)Ljava/lang/Object;") => {
            let action = args[0].clone().to_ref();
            runnerPrint!(runtime, true, 2, "BUILTIN: doPrivileged {}", action);
            try!(invoke_nested(runtime, action.type_ref.clone(), args.clone(), "run", "()Ljava/lang/Object;", false));
        }
        ("java/security/AccessController", "doPrivileged", "(Ljava/security/PrivilegedExceptionAction;)Ljava/lang/Object;") => {
            let action = args[0].clone().to_ref();
            runnerPrint!(runtime, true, 2, "BUILTIN: doPrivileged (ExceptionAction) {}", action);
            try!(invoke_nested(runtime, action.type_ref.clone(), args.clone(), "run", "()Ljava/lang/Object;", false));
        }
        ("java/security/AccessController", "getStackAccessControlContext", "()Ljava/security/AccessControlContext;") => {
            let ret = try!(construct_null_object_by_name(runtime, &"java/security/AccessControlContext"));
            runtime.push_on_stack(ret);
        }
        _ => return Ok(false)
    };
    return Ok(true);
}
