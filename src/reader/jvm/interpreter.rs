extern crate byteorder;

use self::byteorder::{BigEndian, ReadBytesExt};
use reader::builtins::*;
use reader::class_reader::*;
use reader::jvm::construction::*;
use reader::jvm::class_objects::*;
use reader::runner::*;
use reader::util::*;
use std;
use std::io::Cursor;
use std::ops::BitAnd;
use std::ops::BitOr;
use std::ops::BitXor;
use std::rc::Rc;

fn load<F>(desc: &str, index: u8, runtime: &mut Runtime, _t: F) -> Result<(), RunnerError> { // TODO: Type checking
    let loaded = runtime.current_frame.local_variables[index as usize].clone();
    runnerPrint!(runtime, true, 2, "{} {} {}", desc, index, loaded);
    runtime.push_on_stack(loaded);
    return Ok(());
}

fn aload<F, G>(desc: &str, runtime: &mut Runtime, _t: F, converter: G) -> Result<(), RunnerError>
    where G: Fn(Variable) -> Variable
{ // TODO: Type checking
    let index = runtime.pop_from_stack().unwrap().to_int();
    let var = runtime.pop_from_stack().unwrap();
    let array_obj = var.to_arrayobj();
    runnerPrint!(runtime, true, 2, "{} {} {}", desc, index, var);
    if array_obj.is_null {
        let exception = try!(construct_object(runtime, &"java/lang/NullPointerException"));
        return Err(RunnerError::Exception(exception));
    }

    let array = array_obj.elements.borrow();
    if array.len() <= index as usize {
        let exception = try!(construct_object(runtime, &"java/lang/ArrayIndexOutOfBoundsException"));
        return Err(RunnerError::Exception(exception));
    }

    let item = converter(array[index as usize].clone());

    runtime.push_on_stack(item);
    return Ok(());
}

fn store<F>(desc: &str, index: u8, runtime: &mut Runtime, _t: F) -> Result<(), RunnerError> { // TODO: Type checking
    let popped = runtime.pop_from_stack().unwrap();
    runnerPrint!(runtime, true, 2, "{}_{} {}", desc, index, popped);
    while runtime.current_frame.local_variables.len() <= index as usize {
        runtime.current_frame.local_variables.push(Variable::Int(0));
    }
    runtime.current_frame.local_variables[index as usize] = popped;
    return Ok(());
}


fn astore<F>(desc: &str, runtime: &mut Runtime, converter: F) -> Result<(), RunnerError>
    where F: Fn(&Variable) -> Variable
{ // TODO: Type checking
    let value = runtime.pop_from_stack().unwrap();
    let index = runtime.pop_from_stack().unwrap().to_int();
    let var = runtime.pop_from_stack().unwrap();
    let array_obj = var.to_arrayobj();
    runnerPrint!(runtime, true, 2, "{} {} {}", desc, index, var);
    if array_obj.is_null {
        let exception = try!(construct_object(runtime, &"java/lang/NullPointerException"));
        return Err(RunnerError::Exception(exception));
    }

    let mut array = array_obj.elements.borrow_mut();
    if array.len() <= index as usize {
        let exception = try!(construct_object(runtime, &"java/lang/ArrayIndexOutOfBoundsException"));
        return Err(RunnerError::Exception(exception));
    }

    array[index as usize] = converter(&value);
    return Ok(());
}

fn and<F>(a: F, b: F) -> <F as std::ops::BitAnd>::Output where F: BitAnd { a&b }
fn or<F>(a: F, b: F) -> <F as std::ops::BitOr>::Output where F: BitOr { a|b }
fn xor<F>(a: F, b: F) -> <F as std::ops::BitXor>::Output where F: BitXor { a^b }

fn maths_instr<F, G, H, K>(desc: &str, runtime: &mut Runtime, creator: F, extractor: G, operation: H)
    where
        F: Fn(K) -> Variable,
        G: Fn(&Variable) -> K,
        H: Fn(K, K) -> K
{
    let popped1 = runtime.pop_from_stack().unwrap();
    let popped2 = runtime.pop_from_stack().unwrap();
    runnerPrint!(runtime, true, 2, "{} {} {}", desc, popped1, popped2);
    runtime.push_on_stack(creator(operation(extractor(&popped2), extractor(&popped1))));
}

fn maths_instr_2<F, G, H, I, J, K, L>(desc: &str, runtime: &mut Runtime, creator: F, extractor1: G, extractor2: H, operation: I)
    where
        F: Fn(L) -> Variable,
        G: Fn(&Variable) -> J,
        H: Fn(&Variable) -> K,
        I: Fn(K, J) -> L
{
    let popped1 = runtime.pop_from_stack().unwrap();
    let popped2 = runtime.pop_from_stack().unwrap();
    runnerPrint!(runtime, true, 2, "{} {} {}", desc, popped1, popped2);
    runtime.push_on_stack(creator(operation(extractor2(&popped2), extractor1(&popped1))));
}

fn single_pop_instr<F, G, H, I, J>(desc: &str, runtime: &mut Runtime, creator: F, extractor: G, operation: H)
    where
        F: Fn(J) -> Variable,
        G: Fn(&Variable) -> I,
        H: Fn(I) -> J
{
    let popped = runtime.pop_from_stack().unwrap();
    runnerPrint!(runtime, true, 2, "{} {}", desc, popped);
    runtime.push_on_stack(creator(operation(extractor(&popped))));
}

fn vreturn<F, K>(desc: &str, runtime: &mut Runtime, extractor: F) -> Result<bool, RunnerError> where F: Fn(&Variable) -> K {
    let popped = runtime.pop_from_stack().unwrap();
    runnerPrint!(runtime, true, 1, "{} {}", desc, popped);
    extractor(&popped); // Type check
    runtime.current_frame = runtime.previous_frames.pop().unwrap();
    runtime.push_on_stack(popped);
    return Err(RunnerError::Return);
}

pub fn invoke_nested(runtime: &mut Runtime, class: Rc<Class>, args: Vec<Variable>, method_name: &str, method_descriptor: &str, allow_not_found: bool) -> Result<(), RunnerError>{
    let maybe_code = class.cr.get_code(method_name, method_descriptor);
    if maybe_code.is_err() {
        if allow_not_found { return Ok(()) }
            else { try!(Err(maybe_code.err().unwrap())) }
    } else {
        let new_frame = Frame {
            class: Some(class.clone()),
            constant_pool: class.cr.constant_pool.clone(),
            operand_stack: Vec::new(),
            local_variables: args.clone(),
            name: String::from(class.name.clone() + method_name),
            code: maybe_code.unwrap(),
            return_pos: 0,
        };

        runnerPrint!(runtime, true, 1, "INVOKE manual {} {} on {}", method_name, method_descriptor, class.name);
        runtime.previous_frames.push(runtime.current_frame.clone());
        runtime.current_frame = new_frame;
        return do_run_method(runtime);
    }
}

fn invoke(desc: &str, runtime: &mut Runtime, index: u16, with_obj: bool, special: bool) -> Result<(), RunnerError> {
    let debug = true;
    let mut code : Option<Code>;
    let new_frame : Option<Frame>;
    let new_method_name : Option<String>;
    let current_op_stack_size = runtime.current_frame.operand_stack.len();

    {
        let (class_name, method_name, descriptor) = try!(runtime.current_frame.constant_pool.get_method(index));
        new_method_name = Some((*class_name).clone() + "/" + method_name.as_str());
        let (parameters, _return_type) = try!(parse_function_type_descriptor(runtime, descriptor.as_str()));
        let extra_parameter = if with_obj {1} else {0};
        let new_local_variables = runtime.current_frame.operand_stack.split_off(current_op_stack_size - parameters.len() - extra_parameter);

        runnerPrint!(runtime, debug, 1, "{} {} {} {}", desc, class_name, method_name, descriptor);

        if try!(try_builtin(&class_name, &method_name, &descriptor, &new_local_variables, runtime)) {
            return Ok(());
        }

        let mut class = try!(load_class(runtime, class_name.as_str()));

        if with_obj {
            let mut obj = new_local_variables[0].to_ref();

            if obj.is_null {
                return Err(RunnerError::ClassInvalid2(format!("Missing obj ref on local var stack for method on {}", class_name)));
            }

            if special {
                while obj.type_ref.name != *class_name {
                    let new_obj = try!(
                        obj.super_class.borrow().as_ref()
                            .ok_or(RunnerError::ClassInvalid2(format!("Couldn't find class {} in tree for {}", class_name, obj.type_ref.name)))
                    ).clone();
                    obj = new_obj;
                }
            } else {
                obj = get_most_sub_class(obj);
            }

            // Find method
            while { code = obj.type_ref.cr.get_code(method_name.as_str(), descriptor.as_str()).ok(); code.is_none() } {
                if obj.super_class.borrow().is_none() {
                    return Err(RunnerError::ClassInvalid2(format!("Could not find super class of object '{}' that matched method '{}' '{}'", obj, method_name, descriptor)))
                }
                let new_obj = obj.super_class.borrow().clone().unwrap();
                obj = new_obj;
            }
            class = obj.type_ref.clone();
        } else {
            code = Some(try!(class.cr.get_code(method_name.as_str(), descriptor.as_str())));
        }

        new_frame = Some(Frame {
            class: Some(class.clone()),
            constant_pool: class.cr.constant_pool.clone(),
            operand_stack: Vec::new(),
            local_variables: new_local_variables,
            name: new_method_name.unwrap(),
            code: code.unwrap(),
            return_pos: 0,
        });

    }

    runtime.previous_frames.push(runtime.current_frame.clone());
    runtime.current_frame = new_frame.unwrap();
    return Err(RunnerError::Invoke);
}

fn fcmp(desc: &str, runtime: &mut Runtime, is_g: bool) -> Result<(), RunnerError> {
    let pop2 = runtime.pop_from_stack().unwrap().to_float();
    let pop1 = runtime.pop_from_stack().unwrap().to_float();
    runnerPrint!(runtime, true, 2, "{} {} {}", desc, pop1, pop2);
    let ret;
    if pop1.is_nan() || pop2.is_nan() {
        ret = if is_g {1} else {-1}
    } else if pop1 > pop2 {
        ret = 1;
    } else if pop1 == pop2 {
        ret = 0;
    } else {
        ret = -1;
    }
    runtime.push_on_stack(Variable::Int(ret));
    return Ok(());
}

fn ifcmp<F>(desc: &str, runtime: &mut Runtime, buf: &mut Cursor<&Vec<u8>>, cmp: F) -> Result<(), RunnerError>
    where F: Fn(i32) -> bool
{
    let current_position = buf.position() - 1;
    let branch_offset = try!(buf.read_u16::<BigEndian>()) as i16;
    let popped = runtime.pop_from_stack().unwrap();
    runnerPrint!(runtime, true, 2, "{} {} {}", desc, popped, branch_offset);
    if cmp(popped.to_int()) {
        let new_position = (current_position as i64 + branch_offset as i64) as u64;
        runnerPrint!(runtime, true, 2, "BRANCHED from {} to {}", current_position, new_position);
        buf.set_position(new_position);
    }
    return Ok(());
}

fn branch_if<F>(desc: &str, runtime: &mut Runtime, buf: &mut Cursor<&Vec<u8>>, current_position: u64, cmp: F) -> Result<(), RunnerError>
    where F: Fn(&Variable) -> bool
{
    let branch_offset = try!(buf.read_u16::<BigEndian>()) as i16;
    let var = runtime.pop_from_stack().unwrap();
    let compare_result = cmp(&var);
    runnerPrint!(runtime, true, 2, "{} {} {} {}", desc, var, branch_offset, compare_result);
    if compare_result {
        let new_pos = (current_position as i64 + branch_offset as i64) as u64;
        runnerPrint!(runtime, true, 2, "BRANCHED from {} to {}", current_position, new_pos);
        buf.set_position(new_pos);
    }
    return Ok(());
}

fn icmp<F>(desc: &str, runtime: &mut Runtime, buf: &mut Cursor<&Vec<u8>>, cmp: F) -> Result<(), RunnerError>
    where F: Fn(i32, i32) -> bool
{
    let current_position = buf.position() - 1;
    let branch_offset = try!(buf.read_u16::<BigEndian>()) as i16;
    let popped2 = runtime.pop_from_stack().unwrap();
    let popped1 = runtime.pop_from_stack().unwrap();
    runnerPrint!(runtime, true, 2, "{} {} {} {}", desc, popped1, popped2, branch_offset);
    if cmp(popped1.to_int(), popped2.to_int()) {
        let new_position = (current_position as i64 + branch_offset as i64) as u64;
        runnerPrint!(runtime, true, 2, "BRANCHED from {} to {}", current_position, new_position);
        buf.set_position(new_position);
    }
    return Ok(());
}

fn cast<F>(desc: &str, runtime: &mut Runtime, mutator: F)
    where F: Fn(&Variable) -> Variable
{
    let popped = runtime.pop_from_stack().unwrap();
    runnerPrint!(runtime, true, 2, "{} {}", desc, popped);
    runtime.push_on_stack(mutator(&popped));
}

fn ifacmp(desc: &str, runtime: &mut Runtime, buf: &mut Cursor<&Vec<u8>>, should_match: bool) -> Result<(), RunnerError>
{
    let current_position = buf.position() - 1;
    let branch_offset = try!(buf.read_u16::<BigEndian>()) as i16;
    let popped2 = runtime.pop_from_stack().unwrap();
    let popped1 = runtime.pop_from_stack().unwrap();
    runnerPrint!(runtime, true, 2, "{} {} {} {}", desc, popped1, popped2, branch_offset);
    let matching = match popped1 {
        Variable::Reference(ref obj1) => {
            match popped2 {
                Variable::Reference(ref obj2) => {
                    (obj1.is_null && obj2.is_null) || rc_ptr_eq(obj1, obj2)
                },
                _ => false
            }
        },
        Variable::ArrayReference(ref aobj1) => {
            match popped2 {
                Variable::ArrayReference(ref aobj2) => {
                    (aobj1.is_null && aobj2.is_null) || rc_ptr_eq(aobj1, aobj2)
                },
                _ => false
            }
        },
        _ => false
    };
    if should_match == matching {
        let new_position = (current_position as i64 + branch_offset as i64) as u64;
        runnerPrint!(runtime, true, 2, "BRANCHED from {} to {}", current_position, new_position);
        buf.set_position(new_position);
    }
    return Ok(());
}

fn ldc(runtime: &mut Runtime, index: usize) -> Result<(), RunnerError> {
    let maybe_cp_entry = runtime.current_frame.constant_pool.pool.get(&(index as u16)).map(|x| x.clone());
    if maybe_cp_entry.is_none() {
        runnerPrint!(runtime, true, 1, "LDC failed at index {}", index);
        return Err(RunnerError::ClassInvalid2(format!("LDC failed at index {}", index)));
    } else {
        match maybe_cp_entry.as_ref().unwrap() {
            &ConstantPoolItem::CONSTANT_String { index } => {
                let str = try!(runtime.current_frame.constant_pool.get_str(index));
                runnerPrint!(runtime, true, 2, "LDC string {}", str);
                let var = try!(make_string(runtime, str.as_str()));
                runtime.push_on_stack(var);
            }
            &ConstantPoolItem::CONSTANT_Class { index } => {
                let constant_pool_descriptor = try!(runtime.current_frame.constant_pool.get_str(index));
                // Class descriptors are either:
                // "ClassName"
                // or
                // "[[[[I"
                // or
                // "[[[[LClassName;"
                // We first normalise this to a standard descriptor. Note we know it cannot be primitive
                let mut descriptor;
                if constant_pool_descriptor.chars().nth(0).unwrap() == '[' {
                    descriptor = (*constant_pool_descriptor).clone();
                } else {
                    descriptor = 'L'.to_string();
                    descriptor.push_str(constant_pool_descriptor.as_str());
                    descriptor.push(';');
                }
                runnerPrint!(runtime, true, 2, "LDC class {}", descriptor);
                let var = try!(get_class_object_from_descriptor(runtime, descriptor.as_str()));
                runtime.push_on_stack(var);
            }
            &ConstantPoolItem::CONSTANT_Integer { value } => {
                runnerPrint!(runtime, true, 2, "LDC int {}", value as i32);
                runtime.push_on_stack(Variable::Int(value as i32));
            }
            &ConstantPoolItem::CONSTANT_Float { value } => {
                runnerPrint!(runtime, true, 2, "LDC float {}", value as f32);
                runtime.push_on_stack(Variable::Float(value as f32));
            }
            _ => return Err(RunnerError::ClassInvalid2(format!("Unknown constant {:?}", maybe_cp_entry.as_ref().unwrap())))
        }
    }
    return Ok(());
}

pub fn step(runtime: &mut Runtime, name: &str, buf: &mut Cursor<&Vec<u8>>) -> Result<bool, RunnerError> {
    let current_position = buf.position();
    let op_code = try!(buf.read_u8());
    runnerPrint!(runtime, true, 3, "{} {} Op code {}", name, runtime.count, op_code);
    runtime.count+=1;
    match op_code {
        1 => {
            runnerPrint!(runtime, true, 2, "ACONST_NULL");
            let obj = try!(construct_null_object_by_name(runtime, "java/lang/Object"));
            runtime.push_on_stack(obj);
        }
        2...8 => {
            let val = (op_code as i32) - 3;
            runnerPrint!(runtime, true, 2, "ICONST {}", val);
            runtime.push_on_stack(Variable::Int(val));
        }
        9...10 => {
            let val = (op_code as i64) - 9;
            runnerPrint!(runtime, true, 2, "LCONST {}", val);
            runtime.push_on_stack(Variable::Long(val));
        }
        11...13 => {
            let val = (op_code - 11) as f32;
            runnerPrint!(runtime, true, 2, "FCONST {}", val);
            runtime.push_on_stack(Variable::Float(val));
        }
        16 => {
            let byte = try!(buf.read_u8()) as i32;
            runnerPrint!(runtime, true, 2, "BIPUSH {}", byte);
            runtime.push_on_stack(Variable::Int(byte));
        }
        17 => {
            let short = try!(buf.read_u16::<BigEndian>()) as i32;
            runnerPrint!(runtime, true, 2, "SIPUSH {}", short);
            runtime.push_on_stack(Variable::Int(short));
        }
        18 => { // LDC
            let index = try!(buf.read_u8());
            try!(ldc(runtime, index as usize));
        },
        19 => {
            let index = try!(buf.read_u16::<BigEndian>());
            try!(ldc(runtime, index as usize));
        }
        20 => { // LDC2W
            let index = try!(buf.read_u16::<BigEndian>());
            let maybe_cp_entry = runtime.current_frame.constant_pool.pool.get(&(index as u16)).map(|x| x.clone());
            if maybe_cp_entry.is_none() {
                runnerPrint!(runtime, true, 1, "LDC2W failed at index {}", index);
                return Err(RunnerError::ClassInvalid2(format!("LDC2W failed at index {}", index)));
            } else {
                match maybe_cp_entry.as_ref().unwrap() {
                    &ConstantPoolItem::CONSTANT_Long { value } => {
                        runnerPrint!(runtime, true, 2, "LDC2W long {}", value);
                        runtime.push_on_stack(Variable::Long(value as i64));
                    }
                    &ConstantPoolItem::CONSTANT_Double { value } => {
                        runnerPrint!(runtime, true, 2, "LDC2W double {}", value);
                        runtime.push_on_stack(Variable::Double(value));
                    }
                    _ => return Err(RunnerError::ClassInvalid2(format!("Invalid constant for LDC2W {:?}", maybe_cp_entry.as_ref().unwrap())))
                }
            }
        },
        21 => try!(load("ILOAD", try!(buf.read_u8()), runtime, Variable::Int)),
        22 => try!(load("LLOAD", try!(buf.read_u8()), runtime, Variable::Long)),
        23 => try!(load("FLOAD", try!(buf.read_u8()), runtime, Variable::Float)),
        24 => try!(load("DLOAD", try!(buf.read_u8()), runtime, Variable::Double)),
        25 => try!(load("ALOAD", try!(buf.read_u8()), runtime, Variable::Reference)),
        26...29 => try!(load("ILOAD", op_code - 26, runtime, Variable::Int)),
        30...33 => try!(load("LLOAD", op_code - 30, runtime, Variable::Long)),
        34...37 => try!(load("FLOAD", op_code - 34, runtime, Variable::Float)),
        38...41 => try!(load("DLOAD", op_code - 38, runtime, Variable::Double)),
        42...45 => try!(load("ALOAD", op_code - 42, runtime, Variable::Reference)),
        46 => try!(aload("IALOAD", runtime, Variable::Int, |x| x)),
        47 => try!(aload("LALOAD", runtime, Variable::Long, |x| x)),
        48 => try!(aload("FALOAD", runtime, Variable::Float, |x| x)),
        49 => try!(aload("DALOAD", runtime, Variable::Double, |x| x)),
        50 => try!(aload("AALOAD", runtime, Variable::Reference, |x| x)),
        51 => try!(aload("BALOAD", runtime, Variable::Byte, |x| x)),
        52 => try!(aload("CALOAD", runtime, Variable::Char, |x| Variable::Int(Variable::to_int(&x)))),
        53 => try!(aload("SALOAD", runtime, Variable::Short, |x| x)),
        54 => try!(store("ISTORE", try!(buf.read_u8()), runtime, Variable::Int)),
        55 => try!(store("LSTORE", try!(buf.read_u8()), runtime, Variable::Long)),
        56 => try!(store("FSTORE", try!(buf.read_u8()), runtime, Variable::Float)),
        57 => try!(store("DSTORE", try!(buf.read_u8()), runtime, Variable::Double)),
        58 => try!(store("ASTORE", try!(buf.read_u8()), runtime, Variable::Reference)),
        59...62 => try!(store("ISTORE", op_code - 59, runtime, Variable::Int)),
        63...66 => try!(store("LSTORE", op_code - 63, runtime, Variable::Long)),
        67...70 => try!(store("FSTORE", op_code - 67, runtime, Variable::Float)),
        71...74 => try!(store("DSTORE", op_code - 71, runtime, Variable::Double)),
        75...78 => try!(store("ASTORE", op_code - 75, runtime, Variable::Reference)),
        79 => try!(astore("IASTORE", runtime, |x| x.clone())),
        80 => try!(astore("LASTORE", runtime, |x| x.clone())),
        81 => try!(astore("FASTORE", runtime, |x| x.clone())),
        82 => try!(astore("DASTORE", runtime, |x| x.clone())),
        83 => try!(astore("AASTORE", runtime, |x| x.clone())),
        84 => try!(astore("BASTORE", runtime, |x| Variable::Byte(x.to_int() as u8))),
        85 => try!(astore("CASTORE", runtime, |x| Variable::Char(std::char::from_u32((x.to_int() as u32) & 0xFF).unwrap()))),
        86 => try!(astore("SASTORE", runtime, |x| Variable::Short(x.to_int() as i16))),
        87 => {
            let popped = runtime.pop_from_stack().unwrap();
            runnerPrint!(runtime, true, 2, "POP {}", popped);
        }
        88 => {
            let popped = runtime.pop_from_stack().unwrap();
            if popped.is_type_1() {
                let popped2 = runtime.pop_from_stack().unwrap();
                runnerPrint!(runtime, true, 2, "POP2 {} {}", popped, popped2);
            } else {
                runnerPrint!(runtime, true, 2, "POP2 {}", popped);
            }
        }
        89 => {
            let stack_len = runtime.current_frame.operand_stack.len();
            let peek = runtime.current_frame.operand_stack[stack_len - 1].clone();
            runnerPrint!(runtime, true, 2, "DUP {}", peek);
            runtime.push_on_stack(peek);
        }
        90 => {
            let stack_len = runtime.current_frame.operand_stack.len();
            let peek = runtime.current_frame.operand_stack[stack_len - 1].clone();
            runnerPrint!(runtime, true, 2, "DUP_X1 {}", peek);
            runtime.current_frame.operand_stack.insert(stack_len - 2, peek);
        }
        91 => {
            let stack_len = runtime.current_frame.operand_stack.len();
            let peek = runtime.current_frame.operand_stack[stack_len - 1].clone();
            runnerPrint!(runtime, true, 2, "DUP_X2 {}", peek);
            runtime.current_frame.operand_stack.insert(stack_len - 3, peek);
        }
        92 => {
            let stack_len = runtime.current_frame.operand_stack.len();
            let peek1 = runtime.current_frame.operand_stack[stack_len - 1].clone();
            if peek1.is_type_1() {
                let peek2 = runtime.current_frame.operand_stack[stack_len - 2].clone();
                runnerPrint!(runtime, true, 2, "DUP2 {} {}", peek1, peek2);
                runtime.push_on_stack(peek2);
                runtime.push_on_stack(peek1);
            } else {
                runnerPrint!(runtime, true, 2, "DUP2 {}", peek1);
                runtime.push_on_stack(peek1);
            }
        }
        96 => maths_instr("IADD", runtime, Variable::Int, Variable::to_int, i32::wrapping_add),
        97 => maths_instr("LADD", runtime, Variable::Long, Variable::to_long, i64::wrapping_add),
        98 => maths_instr("FADD", runtime, Variable::Float, Variable::to_float, std::ops::Add::add),
        99 => maths_instr("DADD", runtime, Variable::Double, Variable::to_double, std::ops::Add::add),
        100 => maths_instr("ISUB", runtime, Variable::Int, Variable::to_int, i32::wrapping_sub),
        101 => maths_instr("LSUB", runtime, Variable::Long, Variable::to_long, i64::wrapping_sub),
        102 => maths_instr("FSUB", runtime, Variable::Float, Variable::to_float, std::ops::Sub::sub),
        103 => maths_instr("DSUB", runtime, Variable::Double, Variable::to_double, std::ops::Sub::sub),
        104 => maths_instr("IMUL", runtime, Variable::Int, Variable::to_int, i32::wrapping_mul),
        105 => maths_instr("LMUL", runtime, Variable::Long, Variable::to_long, i64::wrapping_mul),
        106 => maths_instr("FMUL", runtime, Variable::Float, Variable::to_float, std::ops::Mul::mul),
        107 => maths_instr("DMUL", runtime, Variable::Double, Variable::to_double, std::ops::Mul::mul),
        108 => maths_instr("IDIV", runtime, Variable::Int, Variable::to_int, i32::wrapping_div),
        109 => maths_instr("LDIV", runtime, Variable::Long, Variable::to_long, i64::wrapping_div),
        110 => maths_instr("FDIV", runtime, Variable::Float, Variable::to_float, std::ops::Div::div),
        111 => maths_instr("DDIV", runtime, Variable::Double, Variable::to_double, std::ops::Div::div),
        112 => maths_instr("IREM", runtime, Variable::Int, Variable::to_int, i32::wrapping_rem),
        113 => maths_instr("LREM", runtime, Variable::Long, Variable::to_long, i64::wrapping_rem),
        114 => maths_instr("FREM", runtime, Variable::Float, Variable::to_float, std::ops::Rem::rem),
        115 => maths_instr("DREM", runtime, Variable::Double, Variable::to_double, std::ops::Rem::rem),
        116 => single_pop_instr("INEG", runtime, Variable::Int, Variable::to_int, |x| 0 - x),
        117 => single_pop_instr("LNEG", runtime, Variable::Long, Variable::to_long, |x| 0 - x),
        118 => single_pop_instr("FNEG", runtime, Variable::Float, Variable::to_float, |x| 0.0 - x),
        119 => single_pop_instr("DNEG", runtime, Variable::Double, Variable::to_double, |x| 0.0 - x),
        120 => maths_instr("ISHL", runtime, Variable::Int, Variable::to_int, |x,y| x << y),
        121 => maths_instr_2("LSHL", runtime, Variable::Long, Variable::to_int, Variable::to_long, |x,y| (x << y) as i64),
        122 => maths_instr("ISHR", runtime, Variable::Int, Variable::to_int, |x,y| x >> y),
        123 => maths_instr_2("LSHR", runtime, Variable::Long, Variable::to_int, Variable::to_long, |x,y| (x >> y) as i64),
        124 => maths_instr("IUSHR", runtime, Variable::Int, Variable::to_int, |x,y| ((x as u32)>>y) as i32),
        125 => maths_instr_2("LUSHR", runtime, Variable::Long, Variable::to_int, Variable::to_long, |x,y| ((x as u64)>>y) as i64),
        126 => maths_instr("IAND", runtime, Variable::Int, Variable::to_int, and),
        127 => maths_instr("LAND", runtime, Variable::Long, Variable::to_long, and),
        128 => maths_instr("IOR", runtime, Variable::Int, Variable::to_int, or),
        129 => maths_instr("LOR", runtime, Variable::Long, Variable::to_long, or),
        130 => maths_instr("IXOR", runtime, Variable::Int, Variable::to_int, xor),
        131 => maths_instr("LXOR", runtime, Variable::Long, Variable::to_long, xor),
        132 => {
            let index = try!(buf.read_u8());
            let constt = try!(buf.read_u8()) as i8;
            runnerPrint!(runtime, true, 2, "IINC {} {}", index, constt);
            let old_val = runtime.current_frame.local_variables[index as usize].to_int();
            runtime.current_frame.local_variables[index as usize] = Variable::Int(old_val + constt as i32);
        }
        133 => cast("I2L", runtime, |x| Variable::Long(x.to_int() as i64)),
        134 => cast("I2F", runtime, |x| Variable::Float(x.to_int() as f32)),
        135 => cast("I2D", runtime, |x| Variable::Double(x.to_int() as f64)),
        136 => single_pop_instr("L2I", runtime, Variable::Int, Variable::to_long, |x| x as i32),
        139 => cast("F2I", runtime, |x| Variable::Int(x.to_float() as i32)),
        140 => cast("F2L", runtime, |x| Variable::Long(x.to_float() as i64)),
        141 => cast("F2D", runtime, |x| Variable::Double(x.to_float() as f64)),
        142 => cast("D2I", runtime, |x| Variable::Int(x.to_double() as i32)),
        143 => cast("D2L", runtime, |x| Variable::Long(x.to_double() as i64)),
        144 => cast("D2F", runtime, |x| Variable::Float(x.to_double() as f32)),
        145 => cast("I2B", runtime, |x| Variable::Byte(x.to_int() as u8)),
        146 => cast("I2C", runtime, |x| Variable::Char(std::char::from_u32(x.to_int() as u32).unwrap_or('\0'))),
        147 => cast("I2S", runtime, |x| Variable::Short(x.to_int() as i16)),
        148 => {
            let pop2 = runtime.pop_from_stack().unwrap().to_long();
            let pop1 = runtime.pop_from_stack().unwrap().to_long();
            runnerPrint!(runtime, true, 2, "LCMP {} {}", pop1, pop2);
            let ret;
            if pop1 > pop2 {
                ret = 1;
            } else if pop1 == pop2 {
                ret = 0;
            } else {
                ret = -1;
            }
            runtime.push_on_stack(Variable::Int(ret));
        }
        149 => try!(fcmp("FCMPG", runtime, true)),
        150 => try!(fcmp("FCMPL", runtime, false)),
        153 => try!(ifcmp("IFEQ", runtime, buf, |x| x == 0)),
        154 => try!(ifcmp("IFNE", runtime, buf, |x| x != 0)),
        155 => try!(ifcmp("IFLT", runtime, buf, |x| x < 0)),
        156 => try!(ifcmp("IFGE", runtime, buf, |x| x >= 0)),
        157 => try!(ifcmp("IFGT", runtime, buf, |x| x > 0)),
        158 => try!(ifcmp("IFLE", runtime, buf, |x| x <= 0)),
        159 => try!(icmp("IF_ICMPEQ", runtime, buf, |x,y| x == y)),
        160 => try!(icmp("IF_ICMPNE", runtime, buf, |x,y| x != y)),
        161 => try!(icmp("IF_ICMPLT", runtime, buf, |x,y| x < y)),
        162 => try!(icmp("IF_ICMPGE", runtime, buf, |x,y| x >= y)),
        163 => try!(icmp("IF_ICMPGT", runtime, buf, |x,y| x > y)),
        164 => try!(icmp("IF_ICMPLE", runtime, buf, |x,y| x <= y)),
        165 => try!(ifacmp("IF_ACMPEQ", runtime, buf, true)),
        166 => try!(ifacmp("IF_ACMPNEQ", runtime, buf, false)),
        167 => {
            let branch_offset = try!(buf.read_u16::<BigEndian>()) as i16;
            let new_pos = (current_position as i64 + branch_offset as i64) as u64;
            runnerPrint!(runtime, true, 2, "BRANCH from {} to {}", current_position, new_pos);
            buf.set_position(new_pos);
        }
        170 => {
            let pos = buf.position();
            buf.set_position((pos + 3) & !3);
            let default = try!(buf.read_u32::<BigEndian>());
            let low = try!(buf.read_u32::<BigEndian>());
            let high = try!(buf.read_u32::<BigEndian>());
            let value_int = runtime.pop_from_stack().unwrap().to_int() as u32;
            runnerPrint!(runtime, true, 2, "TABLESWITCH {} {} {} {}", default, low, high, value_int);
            if value_int < low || value_int > high {
                let new_pos = (current_position as i64 + default as i64) as u64;
                runnerPrint!(runtime, true, 2, "No match so BRANCH from {} to {}", current_position, new_pos);
                buf.set_position(new_pos);
            } else {
                let pos = buf.position();
                buf.set_position(pos + (value_int - low) as u64 * 4);
                let jump = try!(buf.read_u32::<BigEndian>());
                let new_pos = (current_position as i64 + jump as i64) as u64;
                runnerPrint!(runtime, true, 2, "Match so BRANCH from {} to {}", current_position, new_pos);
                buf.set_position(new_pos);
            }
        }
        171 => {
            let pos = buf.position();
            buf.set_position((pos + 3) & !3);
            let default = try!(buf.read_u32::<BigEndian>());
            let npairs = try!(buf.read_u32::<BigEndian>());
            let value_int = runtime.pop_from_stack().unwrap().to_int();
            runnerPrint!(runtime, true, 2, "LOOKUPSWITCH {} {} {}", default, npairs, value_int);
            let mut matched = false;
            for _i in 0..npairs { // TODO: Nonlinear search
                let match_key = try!(buf.read_u32::<BigEndian>()) as i32;
                let offset = try!(buf.read_u32::<BigEndian>()) as i32;
                if match_key == value_int {
                    let new_pos = (current_position as i64 + offset as i64) as u64;
                    runnerPrint!(runtime, true, 2, "Matched so BRANCH from {} to {}", current_position, new_pos);
                    buf.set_position(new_pos);
                    matched = true;
                    break;
                }
            }
            if matched == false {
                let new_pos = (current_position as i64 + default as i64) as u64;
                runnerPrint!(runtime, true, 2, "No match so BRANCH from {} to {}", current_position, new_pos);
                buf.set_position(new_pos);
            }
        }
        172 => { return vreturn("IRETURN", runtime, Variable::can_convert_to_int); }
        173 => { return vreturn("LRETURN", runtime, Variable::to_long); }
        174 => { return vreturn("FRETURN", runtime, Variable::to_float); }
        175 => { return vreturn("DRETURN", runtime, Variable::to_double); }
        176 => { return vreturn("ARETURN", runtime, Variable::is_ref_or_array); }
        177 => { // return
            runnerPrint!(runtime, true, 1, "RETURN");
            runtime.current_frame = runtime.previous_frames.pop().unwrap();
            return Err(RunnerError::Return);
        }
        178 => { // getstatic
            let index = try!(buf.read_u16::<BigEndian>());
            let (class_name, field_name, typ) = try!(runtime.current_frame.constant_pool.get_field(index));
            runnerPrint!(runtime, true, 2, "GETSTATIC {} {} {}", class_name, field_name, typ);
            let mut class_result = try!(load_class(runtime, class_name.as_str()));
            loop {
                {
                    let statics = class_result.statics.borrow();
                    let maybe_static_variable = statics.get(&*field_name);
                    if maybe_static_variable.is_some() {
                        runnerPrint!(runtime, true, 2, "GETSTATIC found {}", maybe_static_variable.unwrap());
                        runtime.push_on_stack(maybe_static_variable.unwrap().clone());
                        break;
                    }
                }
                let maybe_super = class_result.super_class.borrow().clone();
                if maybe_super.is_none() {
                    return Err(RunnerError::ClassInvalid2(format!("Couldn't find static {} in {}", field_name.as_str(), class_name.as_str())));
                }
                class_result = maybe_super.unwrap();
            }
        }
        179 => { // putstatic
            let index = try!(buf.read_u16::<BigEndian>());
            let value = runtime.pop_from_stack().unwrap();
            let (class_name, field_name, typ) = try!(runtime.current_frame.constant_pool.get_field(index));
            runnerPrint!(runtime, true, 2, "PUTSTATIC {} {} {} {}", class_name, field_name, typ, value);
            try!(put_static(runtime, class_name.as_str(), field_name.as_str(), value));
        }
        180 => {
            let field_index = try!(buf.read_u16::<BigEndian>());
            let (class_name, field_name, typ) = try!(runtime.current_frame.constant_pool.get_field(field_index));
            let var = runtime.pop_from_stack().unwrap();
            let obj = var.to_ref();
            let f = try!(get_field(runtime, &obj, class_name.as_str(), field_name.as_str()));
            runnerPrint!(runtime, true, 2, "GETFIELD class:'{}' field:'{}' type:'{}' object:'{}' result:'{}'", class_name, field_name, typ, obj, f);
            runtime.push_on_stack(f);
        }
        181 => {
            let field_index = try!(buf.read_u16::<BigEndian>());
            let (class_name, field_name, typ) = try!(runtime.current_frame.constant_pool.get_field(field_index));
            let value = runtime.pop_from_stack().unwrap();
            let var = runtime.pop_from_stack().unwrap();
            let obj = var.to_ref();
            runnerPrint!(runtime, true, 2, "PUTFIELD {} {} {} {} {}", class_name, field_name, typ, obj, value);
            try!(put_field(runtime, obj, class_name.as_str(), field_name.as_str(), value));
        }
        182 => {
            let index = try!(buf.read_u16::<BigEndian>());
            try!(invoke("INVOKEVIRTUAL", runtime, index, true, false));
        },
        183 => {
            let index = try!(buf.read_u16::<BigEndian>());
            try!(invoke("INVOKESPECIAL", runtime, index, true, true));
        },
        184 => {
            let index = try!(buf.read_u16::<BigEndian>());
            try!(invoke("INVOKESTATIC", runtime, index, false, true));
        }
        185 => {
            let index = try!(buf.read_u16::<BigEndian>());
            let _count = try!(buf.read_u8());
            let _zero = try!(buf.read_u8());
            try!(invoke("INVOKEINTERFACE", runtime, index, true, false));
        }
        187 => {
            let index = try!(buf.read_u16::<BigEndian>());
            let class_name = try!(runtime.current_frame.constant_pool.get_class_name(index));
            runnerPrint!(runtime, true, 2, "NEW {}", class_name);
            let var = try!(construct_object(runtime, class_name.as_str()));
            runtime.push_on_stack(var);
        }
        188 => {
            let atype = try!(buf.read_u8());
            let count = try!(runtime.pop_from_stack().ok_or(RunnerError::ClassInvalid("NEWARRAY POP fail"))).to_int();
            runnerPrint!(runtime, true, 2, "NEWARRAY {} {}", atype, count);

            let var : Variable;
            let type_str : char;
            match atype {
                4 => { var = Variable::Boolean(false); type_str = 'Z'; },
                5 => { var = Variable::Char('\0'); type_str = 'C'; },
                6 => { var = Variable::Float(0.0); type_str = 'F'; },
                7 => { var = Variable::Double(0.0); type_str = 'D'; },
                8 => { var = Variable::Byte(0); type_str = 'B'; },
                9 => { var = Variable::Short(0); type_str = 'S'; },
                10 => { var = Variable::Int(0); type_str = 'I'; },
                11 => { var = Variable::Long(0); type_str = 'J'; },
                _ => return Err(RunnerError::ClassInvalid2(format!("New array type {} unknown", atype)))
            }

            let mut v : Vec<Variable> = Vec::new();
            for _c in 0..count {
                v.push(var.clone());
            }
            let array_obj = try!(construct_primitive_array(runtime, type_str.to_string().as_str(), Some(v)));
            runtime.push_on_stack(array_obj);
        }
        189 => {
            let index = try!(buf.read_u16::<BigEndian>());
            let class_name = try!(runtime.current_frame.constant_pool.get_class_name(index));
            try!(load_class(runtime, class_name.as_str()));
            let class = runtime.classes.get(&*class_name).unwrap().clone();
            let count = try!(runtime.pop_from_stack().ok_or(RunnerError::ClassInvalid("ANEWARRAY count fail"))).to_int();
            runnerPrint!(runtime, true, 2, "ANEWARRAY {} {}", class_name, count);
            let mut v : Vec<Variable> = Vec::new();
            for _c in 0..count {
                v.push(try!(construct_null_object(runtime, class.clone())));
            }
            let array_obj = try!(construct_array(runtime, class, Some(v)));
            runtime.push_on_stack(array_obj);
        }
        190 => {
            let var = runtime.pop_from_stack().unwrap();
            let array_obj = var.to_arrayobj();
            if array_obj.is_null {
                let exception = try!(construct_object(runtime, &"java/lang/NullPointerException"));
                return Err(RunnerError::Exception(exception));
            }
            let len = array_obj.elements.borrow().len();
            runnerPrint!(runtime, true, 2, "ARRAYLEN {} {} {}", var, array_obj.element_type_str, len);
            runtime.push_on_stack(Variable::Int(len as i32));
        }
        192 => {
            let var = runtime.pop_from_stack().unwrap();
            let index = try!(buf.read_u16::<BigEndian>());

            runnerPrint!(runtime, true, 2, "CHECKCAST {} {}", var, index);

            if runtime.current_frame.constant_pool.get_class_name(index).is_err() {
                runnerPrint!(runtime, true, 1, "Missing CP class {}", index);
                return Err(RunnerError::ClassInvalid2(format!("Missing CP class {}", index)));
            }

            // TODO: CHECKCAST (noop)
            runtime.push_on_stack(var);
        }
        193 => {
            let var = runtime.pop_from_stack().unwrap();
            let index = try!(buf.read_u16::<BigEndian>());
            let class_name = try!(runtime.current_frame.constant_pool.get_class_name(index));

            runnerPrint!(runtime, true, 2, "INSTANCEOF {} {}", var, class_name);

            let var_ref = var.to_ref();
            let mut matches = false;
            if !var_ref.is_null {
                let mut obj = get_most_sub_class(var_ref);

                // Search down to find if instance of
                while {matches = obj.type_ref.name == *class_name; obj.super_class.borrow().is_some()} {
                    if matches {
                        break;
                    }
                    let new_obj = obj.super_class.borrow().as_ref().unwrap().clone();
                    obj = new_obj;
                }
            }
            runtime.push_on_stack(Variable::Int(if matches {1} else {0}));
        }
        194 => {
            let var = runtime.pop_from_stack().unwrap();
            runnerPrint!(runtime, true, 2, "MONITORENTER {}", var);
            let _obj = var.to_ref();
            // TODO: Implement monitor
            runnerPrint!(runtime, true, 1, "WARNING: MonitorEnter not implemented");
        },
        195 => {
            let var = runtime.pop_from_stack().unwrap();
            runnerPrint!(runtime, true, 2, "MONITOREXIT {}", var);
            let _obj = var.to_ref();
            // TODO: Implement monitor
            runnerPrint!(runtime, true, 1, "WARNING: MonitorExit not implemented");
        },
        198 => try!(branch_if("IFNULL", runtime, buf, current_position, |x| x.is_null())),
        199 => try!(branch_if("IFNONNULL", runtime, buf, current_position, |x| !x.is_null())),
        _ => return Err(RunnerError::UnknownOpCode(op_code))
    }
    return Ok(false);
}
