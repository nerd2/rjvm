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

fn allocate(count: usize) -> *const u8 {
    let mut v = Vec::with_capacity(count);
    let ptr = v.as_mut_ptr();
    std::mem::forget(v);
    ptr
}

fn deallocate(ptr: *mut u8, count: usize) {
    unsafe {std::mem::drop(Vec::from_raw_parts(ptr, 0, count)) };
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
        ("sun/misc/Unsafe", "allocateMemory", "(J)J") => {
            let size = args[1].to_long();
            runnerPrint!(runtime, true, 2, "BUILTIN: allocateMemory {}", size);
            // TODO: abstract these memory allocations so everything is SAFE? Or continue to allow java to have raw memory access.
            let ptr = allocate(size as usize + 8);
            unsafe { *(ptr as *mut i64) = size; }
            runtime.push_on_stack(Variable::Long(ptr as i64 + 8));
        },
        ("sun/misc/Unsafe", "freeMemory", "(J)V") => {
            let ptr = args[1].to_long() - 8;
            let size = unsafe {*(ptr as *mut i64)} as usize;
            runnerPrint!(runtime, true, 2, "BUILTIN: freeMemory {} {}", ptr, size);
            deallocate(ptr as *const usize as *const _ as *mut _, size);
        },
        ("sun/misc/Unsafe", "putLong", "(JJ)V") => {
            let ptr = args[1].to_long();
            let value = args[3].to_long();
            runnerPrint!(runtime, true, 2, "BUILTIN: putLong {} {}", ptr, value);
            unsafe { *(ptr as *mut u64) = value as u64; }
        },
        ("sun/misc/Unsafe", "getByte", "(J)B") => {
            let ptr = args[1].to_long();
            let byte = unsafe { *(ptr as *mut u8) };
            runnerPrint!(runtime, true, 2, "BUILTIN: getByte {} {}", ptr, byte);
            runtime.push_on_stack(Variable::Byte(byte));
        },
        ("sun/misc/Unsafe", "getObjectVolatile", "(Ljava/lang/Object;J)Ljava/lang/Object;") => { try!(get_at_index(runtime, args, "getObjectVolatile", Variable::to_ref)); }
        ("sun/misc/Unsafe", "getIntVolatile", "(Ljava/lang/Object;J)I") => { try!(get_at_index(runtime, args, "getIntVolatile", Variable::to_int)); }
        ("sun/misc/Unsafe", "getBooleanVolatile", "(Ljava/lang/Object;J)Z") => { try!(get_at_index(runtime, args, "getBooleanVolatile", Variable::to_bool)); }
        ("sun/misc/Unsafe", "getByteVolatile", "(Ljava/lang/Object;J)B") => { try!(get_at_index(runtime, args, "getByteVolatile", Variable::to_byte)); }
        ("sun/misc/Unsafe", "getShortVolatile", "(Ljava/lang/Object;J)S") => { try!(get_at_index(runtime, args, "getShortVolatile", Variable::to_short)); }
        ("sun/misc/Unsafe", "getCharVolatile", "(Ljava/lang/Object;J)C") => { try!(get_at_index(runtime, args, "getCharVolatile", Variable::to_char)); }
        ("sun/misc/Unsafe", "getLongVolatile", "(Ljava/lang/Object;J)J") => { try!(get_at_index(runtime, args, "getLongVolatile", Variable::to_long)); }
        ("sun/misc/Unsafe", "getFloatVolatile", "(Ljava/lang/Object;J)F") => { try!(get_at_index(runtime, args, "getFloatVolatile", Variable::to_float)); }
        ("sun/misc/Unsafe", "getDoubleVolatile", "(Ljava/lang/Object;J)D") => { try!(get_at_index(runtime, args, "getDoubleVolatile", Variable::to_double)); }
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
