use reader::runner::*;
use std::rc::Rc;
use std::rc::Weak;

pub fn register_array_object(_runtime: &mut Runtime, _obj: &Rc<ArrayObject>) {

}

pub fn register_object(runtime: &mut Runtime, obj: &Rc<Object>) {
    println!("Register object {}", &obj);
    runtime.objects.push(Rc::downgrade(obj));
}

fn mark_var(collectable_objects: &mut Vec<Weak<Object>>, var: &Variable) {
    match var {
        &Variable::Reference(ref obj) => {
            collectable_objects.retain(|x| x.upgrade().map(|x| x != *obj).unwrap_or(false));
        },
        &Variable::ArrayReference(ref _obj) => {

        },
        _ => {}
    }
}

fn mark_frame(collectable_objects: &mut Vec<Weak<Object>>, frame: &Frame) {
    for var in &frame.local_variables {
        mark_var(collectable_objects, var);
    }

    for var in &frame.operand_stack {
        mark_var(collectable_objects, var);
    }
}

pub fn gc_hint_run(runtime: &mut Runtime) {
    let mut collectable_objects = runtime.objects.clone();
    mark_frame(&mut collectable_objects, &runtime.current_frame);
    for frame in &runtime.previous_frames {
        mark_frame(&mut collectable_objects, frame);
    }

    for obj in &collectable_objects {
        let mut maybe_obj_ref = obj.upgrade();
        while maybe_obj_ref.is_some() {
            let obj_ref = maybe_obj_ref.unwrap();
            obj_ref.members.borrow_mut().clear();
            maybe_obj_ref = obj_ref.super_class.borrow().clone();
            *obj_ref.super_class.borrow_mut() = None;
        }
    }
}