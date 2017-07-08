package java.lang;

import java.io.InputStream;
import java.io.ObjectStreamField;
import java.io.Serializable;
import java.lang.ClassValue.ClassValueMap;
import java.lang.annotation.Annotation;
import java.lang.ref.SoftReference;
import java.lang.reflect.AnnotatedElement;
import java.lang.reflect.AnnotatedType;
import java.lang.reflect.Array;
import java.lang.reflect.Constructor;
import java.lang.reflect.Executable;
import java.lang.reflect.Field;
import java.lang.reflect.GenericArrayType;
import java.lang.reflect.GenericDeclaration;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.Proxy;
import java.lang.reflect.Type;
import java.lang.reflect.TypeVariable;
import java.net.URL;
import java.security.AccessController;
import java.security.CodeSource;
import java.security.Permissions;
import java.security.PrivilegedAction;
import java.security.ProtectionDomain;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collection;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Iterator;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.Objects;
import java.util.Set;
import java.util.Map.Entry;

public final class Class<T> implements Serializable, GenericDeclaration, Type, AnnotatedElement {
    private final String name;
    private final boolean isInterface;
    private final boolean isArray;
    private final boolean isPrimitive;

    public Class(String name, boolean isInterface, boolean isArray, boolean isPrimitive) {
        this.name = name;
        this.isInterface = isInterface;
        this.isArray = isArray;
        this.isPrimitive = isPrimitive;
    }

    public String toString() {
        return (this.isInterface()?"interface ":(this.isPrimitive()?"":"class ")) + this.getName();
    }

    public String getName() {
        return this.name;
    }

    public boolean isInterface() { return this.isInterface; }

    public boolean isArray() { return this.isArray; }

    public boolean isPrimitive() { return this.isPrimitive; }

    public TypeVariable<Class<T>>[] getTypeParameters() {
        return (TypeVariable[])(new TypeVariable[0]);
    }

    public Annotation[] getDeclaredAnnotations() {
        return null;
    }

    public <T extends Annotation> T getAnnotation(Class<T> var1) {
        return null;
    }

    public Annotation[] getAnnotations() {
        return null;
    }

    public boolean desiredAssertionStatus() {
        return false;
    }

}
