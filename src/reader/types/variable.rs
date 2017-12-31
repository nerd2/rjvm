use reader::jvm::construction::*;
use reader::runner::*;
use reader::util::*;
use std::fmt;
use std::rc::Rc;

#[derive(Clone, PartialEq)]
pub enum Variable {
    Byte(u8),
    Char(char),
    Double(f64),
    Float(f32),
    Int(i32),
    Long(i64),
    Short(i16),
    Boolean(bool),
    Reference(Rc<Class>, Option<Rc<Object>>),
    ArrayReference(Rc<ArrayObject>),
    InterfaceReference(Rc<Object>),
    UnresolvedReference(String),
}

impl Variable {
    pub fn to_bool(&self) -> bool {
        match self {
            &Variable::Boolean(ref x) => {
                return *x;
            },
            &Variable::Int(ref x) => {
                return *x != 0;
            },
            _ => {
                panic!("Couldn't convert to bool");
            }
        }
    }
    pub fn to_char(&self) -> char {
        match self {
            &Variable::Char(ref x) => {
                return *x;
            },
            _ => {
                panic!("Couldn't convert to char");
            }
        }
    }
    pub fn to_byte(&self) -> u8 {
        match self {
            &Variable::Int(ref x) => {
                return *x as u8;
            },
            &Variable::Byte(ref x) => {
                return *x;
            },
            _ => {
                panic!("Couldn't convert to byte");
            }
        }
    }
    pub fn to_short(&self) -> i16 {
        match self {
            &Variable::Int(ref x) => {
                return *x as i16;
            },
            &Variable::Short(ref x) => {
                return *x;
            },
            _ => {
                panic!("Couldn't convert to short");
            }
        }
    }
    pub fn to_int(&self) -> i32 {
        match self {
            &Variable::Boolean(ref x) => {
                return if *x { 1 } else { 0 };
            },
            &Variable::Char(ref x) => {
                return *x as i32;
            },
            &Variable::Byte(ref x) => {
                return *x as i32;
            },
            &Variable::Short(ref x) => {
                return *x as i32;
            },
            &Variable::Int(ref x) => {
                return *x;
            },
            _ => {
                panic!("Couldn't convert to int");
            }
        }
    }

    pub fn to_long(&self) -> i64 {
        match self {
            &Variable::Long(ref x) => {
                return *x;
            },
            _ => {
                panic!("Couldn't convert to long");
            }
        }
    }
    pub fn to_float(&self) -> f32 {
        match self {
            &Variable::Float(ref x) => {
                return *x;
            },
            _ => {
                panic!("Couldn't convert to float");
            }
        }
    }
    pub fn to_double(&self) -> f64 {
        match self {
            &Variable::Double(ref x) => {
                return *x;
            },
            _ => {
                panic!("Couldn't convert to double");
            }
        }
    }
    pub fn to_ref_type(&self) -> Rc<Class> {
        match self {
            &Variable::Reference(ref class, ref _obj) => {
                return class.clone();
            },
            _ => {
                panic!("Couldn't convert to reference");
            }
        }
    }
    pub fn to_ref(&self) -> Option<Rc<Object>> {
        match self {
            &Variable::Reference(ref _class, ref obj) => {
                return obj.clone();
            },
            _ => {
                panic!("Couldn't convert '{}' to reference", self);
            }
        }
    }
    pub fn is_reference(&self) -> bool {
        match self {
            &Variable::Reference(ref _class, ref _obj) => {
                return true;
            },
            _ => {
                return false;
            }
        }
    }
    pub fn is_array_reference(&self) -> bool {
        match self {
            &Variable::ArrayReference(ref _array) => {
                return true;
            },
            _ => {
                return false;
            }
        }
    }
    pub fn is_ref_or_array(&self) -> bool {
        match self {
            &Variable::Reference(ref _class, ref _obj) => {
                return true;
            },
            &Variable::ArrayReference(ref _array) => {
                return true;
            },
            _ => {
                panic!("Couldn't convert '{}' to reference or array", self);
            }
        }
    }
    pub fn is_null(&self) -> bool {
        match self {
            &Variable::Reference(ref _class, ref obj) => {
                return obj.is_none();
            },
            &Variable::ArrayReference(ref array) => {
                return array.is_null;
            },
            &Variable::UnresolvedReference(ref _x) => {
                return true;
            },
            _ => {
                panic!("Couldn't check if primitive '{}' is null", self);
            }
        }
    }
    pub fn to_arrayobj(&self) -> Rc<ArrayObject> {
        match self {
            &Variable::ArrayReference(ref array) => {
                return array.clone();
            },
            _ => {
                panic!("Couldn't convert to reference");
            }
        }
    }
    pub fn is_type_1(&self) -> bool {
        match self {
            &Variable::Long(_x) => {
                return false;
            },
            &Variable::Double(_y) => {
                return false;
            },
            _ => {
                return true;
            }
        }
    }
    pub fn can_convert_to_int(&self) -> bool {
        return match self {
            &Variable::Boolean(_x) => true,
            &Variable::Byte(_x) => true,
            &Variable::Short(_x) => true,
            &Variable::Char(_x) => true,
            &Variable::Int(_x) => true,
            _ => false,
        }
    }
    pub fn is_primitive(&self) -> bool {
        return match self {
            &Variable::Reference(ref _class, ref _x) => false,
            &Variable::ArrayReference(ref _x) => false,
            &Variable::InterfaceReference(ref _x) => false,
            &Variable::UnresolvedReference(ref _x) => false,
            _ => true,
        }
    }

    pub fn is_unresolved(&self) -> bool {
        return match self {
            &Variable::UnresolvedReference(ref _x) => true,
            _ => false,
        }
    }

    pub fn get_unresolved_type_name(&self) -> String {
        return match self {
            &Variable::UnresolvedReference(ref type_name) => type_name.clone(),
            _ => panic!("Cannot get unresolved type name of {}", self),
        }
    }

    pub fn hash_code(&self, runtime: &mut Runtime) -> Result<i32, RunnerError> {
        match self {
            &Variable::Reference(ref _class, ref obj) => {
                if obj.is_none() {
                    let exception = try!(construct_object(runtime, &"java/lang/NullPointerException"));
                    return Err(RunnerError::Exception(exception));
                } else {
                    return Ok(obj.as_ref().unwrap().code());
                }
            },
            &Variable::ArrayReference(ref obj) => {
                if obj.is_null {
                    let exception = try!(construct_object(runtime, &"java/lang/NullPointerException"));
                    return Err(RunnerError::Exception(exception));
                } else {
                    return Ok(obj.code);
                }
            },
            _ => {
                panic!("Called hashcode on primitive type");
            }
        };
    }

    pub fn get_descriptor(&self) -> String {
        let mut ret = String::new();
        match self {
            &Variable::Byte(_v) => {ret.push('B');},
            &Variable::Char(_v) => {ret.push('C');},
            &Variable::Double(_v) => {ret.push('D');},
            &Variable::Float(_v) => {ret.push('F');},
            &Variable::Int(_v) => {ret.push('I');},
            &Variable::Long(_v) => {ret.push('J');},
            &Variable::Short(_v) => {ret.push('S');},
            &Variable::Boolean(_v) => {ret.push('Z');},
            &Variable::Reference(ref class, ref _obj) => {return generate_class_descriptor(class); },
            &Variable::ArrayReference(ref array_obj) => {
                ret.push('[');
                if array_obj.element_type_ref.is_some() {
                    ret.push_str(generate_class_descriptor(array_obj.element_type_ref.as_ref().unwrap()).as_str());
                } else {
                    ret.push_str(array_obj.element_type_str.as_str());
                }
            },
            &Variable::UnresolvedReference(ref class_name) => {
                ret.push('L');
                ret.push_str(class_name.as_str());
                ret.push(';');
            },
            _ => {panic!("Type not covered");}
        }
        return ret;
    }

    pub fn display(&self) -> String {
        return match self {
            &Variable::Reference(ref class, ref obj) =>
                if obj.is_some() {
                    format!("Reference of type {} with content {}", class.name, obj.as_ref().unwrap())
                } else {
                    format!("<null of type {}>", class.name)
                },
            &Variable::ArrayReference(ref array) => format!("ArrayReference {}", array),
            &Variable::InterfaceReference(ref interface) => format!("InterfaceReference {}", interface),
            &Variable::Boolean(x) => format!("Boolean {}", x),
            &Variable::Byte(x) => format!("Byte {}", x),
            &Variable::Short(x) => format!("Short {}", x),
            &Variable::Char(x) => format!("Char {}", x),
            &Variable::Int(x) => format!("Int {}", x),
            &Variable::Double(x) => format!("Double {}", x),
            &Variable::Float(x) => format!("Float {}", x),
            &Variable::Long(x) => format!("Long {}", x),
            &Variable::UnresolvedReference(ref name) => format!("UnresolvedReference {}", name),
        }
    }

    pub fn extract_string(&self) -> String {
        match self {
            &Variable::Reference(ref class, ref obj) => {
                match class.name.as_str() {
                    "java/lang/String" => {
                        return string_to_string(obj.as_ref().unwrap());
                    },
                    _ => {panic!("{} is not a string", self);}
                }
            }
            _ => {panic!("{} is not a string", self);}
        }
    }
}
impl fmt::Display for Variable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return write!(f, "{}", self.display());
    }
}
impl fmt::Debug for Variable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return write!(f, "{}", self.display());
    }
}
