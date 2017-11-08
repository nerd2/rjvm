use reader::runner::*;
use reader::jvm::class_objects::*;
use reader::util::*;
use std;
use std::rc::Rc;

fn get_at_index<F, G>(runtime: &mut Runtime, args: &Vec<Variable>, desc: &str, validator: F)-> Result<(), RunnerError>
    where F: Fn(&Variable) -> G
{
    let obj = args[1].clone().to_ref();
    let offset = args[2].to_long();
    let ret = try!(obj.get_at_index(offset));
    validator(&ret);
    runnerPrint!(runtime, true, 2, "BUILTIN: {} {} {} {}", desc, obj, offset, ret);
    runtime.push_on_stack(ret);
    return Ok(());
}

fn compare_and_swap<F, G>(runtime: &mut Runtime, args: &Vec<Variable>, desc: &str, extractor: F, second_offset: usize) -> Result<(), RunnerError>
    where F: Fn(&Variable) -> G,
          G: std::cmp::PartialEq,
          G: std::fmt::Display
{
    let obj = args[1].clone().to_ref();
    let class = args[1].clone().to_ref_type();
    let offset = args[2].to_long(); // 2 slots :(
    let expected = extractor(&args[4].clone());
    let swap = args[5 + second_offset].clone();

    let field = &class.cr.fields[offset as usize];
    let name_string = try!(class.cr.constant_pool.get_str(field.name_index));
    let mut members = obj.members.borrow_mut();
    runnerPrint!(runtime, true, 2, "BUILTIN: {} {} {} {} {} {}", desc, obj, offset, name_string, expected, swap);
    let current = extractor(members.get(&*name_string).unwrap());
    runnerPrint!(runtime, true, 2, "BUILTIN: {} {} {} {} {} {}", desc, obj, offset, current, expected, swap);
    let ret;
    if current == expected {
        runnerPrint!(runtime, true, 3, "BUILTIN: {} swapped", desc);
        members.insert((*name_string).clone(), swap);
        ret = true;
    } else {
        ret = false;
    }
    runtime.push_on_stack(Variable::Boolean(ret));
    return Ok(());
}

pub fn try_builtin(class_name: &Rc<String>, method_name: &Rc<String>, descriptor: &Rc<String>, args: &Vec<Variable>, runtime: &mut Runtime) -> Result<bool, RunnerError> {
    match (class_name.as_str(), method_name.as_str(), descriptor.as_str()) {
        ("sun/misc/Unsafe", "registerNatives", "()V") => {return Ok(true)},
        ("sun/misc/Unsafe", "arrayBaseOffset", "(Ljava/lang/Class;)I") => {
            runnerPrint!(runtime, true, 2, "BUILTIN: arrayBaseOffset");
            runtime.push_on_stack(Variable::Int(0));
        },
        ("sun/misc/Unsafe", "objectFieldOffset", "(Ljava/lang/reflect/Field;)J") => {
            let obj = args[1].clone().to_ref();
            let slot = try!(get_field(runtime, &obj, &"java/lang/reflect/Field", "slot")).to_int();

            runnerPrint!(runtime, true, 2, "BUILTIN: objectFieldOffset {} {}", obj, slot);
            runtime.push_on_stack(Variable::Long(slot as i64));
        },
        ("sun/misc/Unsafe", "arrayIndexScale", "(Ljava/lang/Class;)I") => {
            runnerPrint!(runtime, true, 2, "BUILTIN: arrayIndexScale");
            runtime.push_on_stack(Variable::Int(1));
        },
        ("sun/misc/Unsafe", "addressSize", "()I") => {
            runnerPrint!(runtime, true, 2, "BUILTIN: addressSize");
            runtime.push_on_stack(Variable::Int(4));
        },
        ("sun/misc/Unsafe", "pageSize", "()I") => {
            runnerPrint!(runtime, true, 2, "BUILTIN: pageSize");
            runtime.push_on_stack(Variable::Int(4096));
        },
        ("sun/misc/Unsafe", "compareAndSwapObject", "(Ljava/lang/Object;JLjava/lang/Object;Ljava/lang/Object;)Z") => { try!(compare_and_swap(runtime, args, "compareAndSwapObject", Variable::to_ref, 0));}
        ("sun/misc/Unsafe", "compareAndSwapInt", "(Ljava/lang/Object;JII)Z") => { try!(compare_and_swap(runtime, args, "compareAndSwapInt", Variable::to_int, 0));}
        ("sun/misc/Unsafe", "compareAndSwapLong", "(Ljava/lang/Object;JJJ)Z") => { try!(compare_and_swap(runtime, args, "compareAndSwapLong", Variable::to_long, 1));}
        ("sun/misc/VM", "initialize", "()V") => {}
        ("sun/reflect/Reflection", "getCallerClass", "()Ljava/lang/Class;") => {
            let class = runtime.previous_frames[runtime.previous_frames.len()-1].class.clone().unwrap();
            let var = try!(get_class_object_from_descriptor(runtime, type_name_to_descriptor(&class.name).as_str()));
            runnerPrint!(runtime, true, 2, "BUILTIN: getCallerClass {}", var);
            runtime.push_on_stack(var);
        }
        _ => return Ok(false)
    };
    return Ok(true);
}
