use reader::class_reader::*;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub struct ConstantPool {
    pub pool: HashMap<u16, ConstantPoolItem>,
}

impl ConstantPool {
    pub fn new() -> ConstantPool {
        ConstantPool {pool: HashMap::new()}
    }

    pub fn get_str(&self, index: u16) -> Result<Rc<String>, ClassReadError> {
        let maybe_cp_entry = self.pool.get(&index);
        if maybe_cp_entry.is_none() {
            debugPrint!(true, 2, "Constant pool item at index {} is not present", index);
            return Err(ClassReadError::Parse);
        } else {
            match *maybe_cp_entry.unwrap() {
                ConstantPoolItem::CONSTANT_Utf8(ref s) => {
                    return Ok(s.clone());
                }
                _ => {
                    debugPrint!(true, 2, "Constant pool item at index {} is not UTF8, actually {:?}", index, maybe_cp_entry.unwrap());
                    return Err(ClassReadError::Parse);
                }
            }
        }
    }

    pub fn get_class_name(&self, index:u16) -> Result<Rc<String>, ClassReadError> {
        let maybe_cp_entry = self.pool.get(&index);
        if maybe_cp_entry.is_none() {
            debugPrint!(true, 4, "Constant pool item at index {} does not exist", index);
            return Err(ClassReadError::Parse);
        } else {
            match *maybe_cp_entry.unwrap() {
                ConstantPoolItem::CONSTANT_Class {index: name_index} => {
                    return self.get_str(name_index);
                }
                _ => {
                    debugPrint!(true, 4, "Constant pool item at index {} is not a class, actually {:?}", index, maybe_cp_entry.unwrap());
                    return Err(ClassReadError::Parse);
                }
            }
        }
    }

    pub fn get_name_and_type(&self, index: u16) -> Result<(Rc<String>, Rc<String>), ClassReadError> {
        debugPrint!(false, 5, "{}", index);

        let maybe_cp_entry = self.pool.get(&index);
        if maybe_cp_entry.is_none() {
            debugPrint!(true, 1, "Missing CP name & type {}", index);
            return Err(ClassReadError::Parse2(format!("Missing CP name & type {}", index)));
        } else {
            match *maybe_cp_entry.unwrap() {
                ConstantPoolItem::CONSTANT_NameAndType {name_index, descriptor_index} => {
                    debugPrint!(false, 4, "name_index: {}, descriptor_index: {}", name_index, descriptor_index);

                    let name_str = try!(self.get_str(name_index));
                    let type_str = try!(self.get_str(descriptor_index));
                    return Ok((name_str, type_str));
                }
                _ => {
                    return Err(ClassReadError::Parse2(format!("Index {} is not a name and type", index)));
                }
            }
        }
    }

    pub fn get_field(&self, index: u16) -> Result<(Rc<String>, Rc<String>, Rc<String>), ClassReadError> {
        debugPrint!(false, 5, "{}", index);
        let maybe_cp_entry = self.pool.get(&index);
        if maybe_cp_entry.is_none() {
            return Err(ClassReadError::Parse2(format!( "Missing CP field {}", index)));
        } else {
            match *maybe_cp_entry.unwrap() {
                ConstantPoolItem::CONSTANT_Fieldref{class_index, name_and_type_index} => {
                    let class_str = try!(self.get_class_name(class_index));
                    let (name_str, type_str) = try!(self.get_name_and_type(name_and_type_index));
                    return Ok((class_str, name_str, type_str));
                }
                _ => {
                    return Err(ClassReadError::Parse2(format!("Index {} is not a field {:?}", index, *maybe_cp_entry.unwrap())));
                }
            }
        }
    }

    pub fn get_method(&self, index: u16) -> Result<(Rc<String>, Rc<String>, Rc<String>), ClassReadError> {
        debugPrint!(false, 5, "{}", index);
        let maybe_cp_entry = self.pool.get(&index);
        if maybe_cp_entry.is_none() {
            debugPrint!(true, 1, "Missing CP method {}", index);
            return Err(ClassReadError::Parse2(format!("Missing CP method {}", index)));
        } else {
            match *maybe_cp_entry.unwrap() {
                ConstantPoolItem::CONSTANT_Methodref {class_index, name_and_type_index} => {
                    let class_str = try!(self.get_class_name(class_index));
                    let (name_str, type_str) = try!(self.get_name_and_type(name_and_type_index));
                    return Ok((class_str, name_str, type_str));
                }
                ConstantPoolItem::CONSTANT_InterfaceMethodref {class_index, name_and_type_index} => {
                    let class_str = try!(self.get_class_name(class_index));
                    let (name_str, type_str) = try!(self.get_name_and_type(name_and_type_index));
                    return Ok((class_str, name_str, type_str));
                }
                _ => {
                    return Err(ClassReadError::Parse2(format!("Index {} is not a method", index)));
                }
            }
        }
    }

}