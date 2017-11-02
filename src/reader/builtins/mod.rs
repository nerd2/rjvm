mod java_lang;
mod java_other;
mod sun;

use reader::runner::*;
use std::rc::Rc;

pub fn try_builtin(class_name: &Rc<String>, method_name: &Rc<String>, descriptor: &Rc<String>, args: &Vec<Variable>, runtime: &mut Runtime) -> Result<bool, RunnerError> {
    runnerPrint!(runtime, true, 4, "try_builtin {} {} {}", class_name, method_name, descriptor);

    if try!(java_lang::try_builtin(class_name, method_name, descriptor, args, runtime)) {
        return Ok((true));
    }
    if try!(java_other::try_builtin(class_name, method_name, descriptor, args, runtime)) {
        return Ok((true));
    }
    if try!(sun::try_builtin(class_name, method_name, descriptor, args, runtime)) {
        return Ok((true));
    }

    return Ok(false);
}
