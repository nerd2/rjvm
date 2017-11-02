use reader::jvm::interpreter::*;
use reader::runner::*;
use std::rc::Rc;

pub fn try_builtin(class_name: &Rc<String>, method_name: &Rc<String>, descriptor: &Rc<String>, args: &Vec<Variable>, runtime: &mut Runtime) -> Result<bool, RunnerError> {
    match (class_name.as_str(), method_name.as_str(), descriptor.as_str()) {
        ("java/net/InetAddress", "init", "()V") => {}
        ("java/net/InetAddressImplFactory", "isIPv6Supported", "()Z") => { runtime.push_on_stack(Variable::Boolean(false)); }
        ("java/util/concurrent/atomic/AtomicLong", "VMSupportsCS8", "()Z") => { runtime.push_on_stack(Variable::Boolean(false)); }
        ("java/security/AccessController", "doPrivileged", "(Ljava/security/PrivilegedAction;)Ljava/lang/Object;") => {
            let action = args[0].clone().to_ref();
            runnerPrint!(runtime, true, 2, "BUILTIN: doPrivileged {}", action);
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
