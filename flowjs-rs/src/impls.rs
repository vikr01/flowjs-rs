//! Built-in Flow trait implementations for Rust standard types.

use crate::{Config, Dummy, Flow, TypeVisitor};

// Wrapper types: delegate all methods to the inner type T
macro_rules! impl_wrapper {
    ($($t:tt)*) => {
        $($t)* {
            type WithoutGenerics = Self;
            type OptionInnerType = Self;
            fn name(cfg: &Config) -> String { <T as crate::Flow>::name(cfg) }
            fn inline(cfg: &Config) -> String { <T as crate::Flow>::inline(cfg) }
            fn inline_flattened(cfg: &Config) -> String { <T as crate::Flow>::inline_flattened(cfg) }
            fn visit_dependencies(v: &mut impl TypeVisitor)
            where
                Self: 'static,
            {
                <T as crate::Flow>::visit_dependencies(v);
            }
            fn visit_generics(v: &mut impl TypeVisitor)
            where
                Self: 'static,
            {
                <T as crate::Flow>::visit_generics(v);
                v.visit::<T>();
            }
            fn decl(_: &Config) -> String { panic!("wrapper type cannot be declared") }
            fn decl_concrete(_: &Config) -> String { panic!("wrapper type cannot be declared") }
        }
    };
}

// Shadow types: delegate to a different impl
macro_rules! impl_shadow {
    (as $s:ty: $($impl:tt)*) => {
        $($impl)* {
            type WithoutGenerics = <$s as crate::Flow>::WithoutGenerics;
            type OptionInnerType = <$s as crate::Flow>::OptionInnerType;
            fn ident(cfg: &Config) -> String { <$s as crate::Flow>::ident(cfg) }
            fn name(cfg: &Config) -> String { <$s as crate::Flow>::name(cfg) }
            fn inline(cfg: &Config) -> String { <$s as crate::Flow>::inline(cfg) }
            fn inline_flattened(cfg: &Config) -> String { <$s as crate::Flow>::inline_flattened(cfg) }
            fn visit_dependencies(v: &mut impl crate::TypeVisitor)
            where
                Self: 'static,
            {
                <$s as crate::Flow>::visit_dependencies(v);
            }
            fn visit_generics(v: &mut impl crate::TypeVisitor)
            where
                Self: 'static,
            {
                <$s as crate::Flow>::visit_generics(v);
            }
            fn decl(cfg: &Config) -> String { <$s as crate::Flow>::decl(cfg) }
            fn decl_concrete(cfg: &Config) -> String { <$s as crate::Flow>::decl_concrete(cfg) }
            fn output_path() -> Option<std::path::PathBuf> { <$s as crate::Flow>::output_path() }
        }
    };
}

macro_rules! impl_flow_primitive {
    ($rust_ty:ty, $flow_ty:expr) => {
        impl Flow for $rust_ty {
            type WithoutGenerics = Self;
            type OptionInnerType = Self;
            fn name(_: &Config) -> String {
                $flow_ty.to_owned()
            }
            fn inline(cfg: &Config) -> String {
                <Self as crate::Flow>::name(cfg)
            }
        }
    };
}

// Primitives
impl_flow_primitive!(bool, "boolean");
impl_flow_primitive!(i8, "number");
impl_flow_primitive!(i16, "number");
impl_flow_primitive!(i32, "number");
impl_flow_primitive!(u8, "number");
impl_flow_primitive!(u16, "number");
impl_flow_primitive!(u32, "number");
impl_flow_primitive!(i64, "number");
impl_flow_primitive!(u64, "number");
impl_flow_primitive!(i128, "number");
impl_flow_primitive!(u128, "number");
impl_flow_primitive!(f32, "number");
impl_flow_primitive!(f64, "number");
impl_flow_primitive!(char, "string");
impl_flow_primitive!(String, "string");
impl_flow_primitive!(str, "string");
impl_flow_primitive!((), "void");

// Option<T> → ?T
impl<T: Flow> Flow for Option<T> {
    type WithoutGenerics = Self;
    type OptionInnerType = T;
    const IS_OPTION: bool = true;

    fn name(cfg: &Config) -> String {
        format!("?{}", T::name(cfg))
    }
    fn inline(cfg: &Config) -> String {
        format!("?{}", T::inline(cfg))
    }
    fn visit_dependencies(v: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
        <T as crate::Flow>::visit_dependencies(v);
    }
    fn visit_generics(v: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
        <T as crate::Flow>::visit_generics(v);
        v.visit::<T>();
    }
}

// Vec<T> → $ReadOnlyArray<T>
impl<T: Flow> Flow for Vec<T> {
    type WithoutGenerics = Vec<Dummy>;
    type OptionInnerType = Self;

    fn ident(_: &Config) -> String {
        "$ReadOnlyArray".to_owned()
    }
    fn name(cfg: &Config) -> String {
        format!("$ReadOnlyArray<{}>", T::name(cfg))
    }
    fn inline(cfg: &Config) -> String {
        format!("$ReadOnlyArray<{}>", T::inline(cfg))
    }
    fn visit_dependencies(v: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
        <T as crate::Flow>::visit_dependencies(v);
    }
    fn visit_generics(v: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
        <T as crate::Flow>::visit_generics(v);
        v.visit::<T>();
    }
}

// &[T] → $ReadOnlyArray<T>
impl_shadow!(as Vec<T>: impl<T: Flow> Flow for [T]);

// Box<T> → T
impl_wrapper!(impl<T: Flow + ?Sized> Flow for Box<T>);

// &T → T
impl_wrapper!(impl<'a, T: Flow + ?Sized> Flow for &'a T);

// std::collections::HashMap → { [key: K]: V }
impl<K: Flow, V: Flow> Flow for std::collections::HashMap<K, V> {
    type WithoutGenerics = std::collections::HashMap<Dummy, Dummy>;
    type OptionInnerType = Self;

    fn ident(_: &Config) -> String {
        panic!()
    }
    fn name(cfg: &Config) -> String {
        format!("{{ [key: {}]: {} }}", K::name(cfg), V::name(cfg))
    }
    fn inline(cfg: &Config) -> String {
        format!("{{ [key: {}]: {} }}", K::inline(cfg), V::inline(cfg))
    }
    fn inline_flattened(cfg: &Config) -> String {
        format!("({})", Self::inline(cfg))
    }
    fn visit_dependencies(v: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
        K::visit_dependencies(v);
        V::visit_dependencies(v);
    }
    fn visit_generics(v: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
        K::visit_generics(v);
        v.visit::<K>();
        V::visit_generics(v);
        v.visit::<V>();
    }
}

// std::collections::BTreeMap → { [key: K]: V }
impl_shadow!(as std::collections::HashMap<K, V>: impl<K: Flow, V: Flow> Flow for std::collections::BTreeMap<K, V>);

// HashSet → $ReadOnlyArray
impl_shadow!(as Vec<T>: impl<T: Flow> Flow for std::collections::HashSet<T>);

// BTreeSet → $ReadOnlyArray
impl_shadow!(as Vec<T>: impl<T: Flow> Flow for std::collections::BTreeSet<T>);

// Tuples
macro_rules! impl_flow_tuples {
    ( impl $($i:ident),* ) => {
        impl<$($i: Flow),*> Flow for ($($i,)*) {
            type WithoutGenerics = (Dummy, );
            type OptionInnerType = Self;
            fn name(cfg: &Config) -> String {
                let parts: Vec<String> = vec![$($i::name(cfg)),*];
                format!("[{}]", parts.join(", "))
            }
            fn inline(_: &Config) -> String {
                panic!("tuple cannot be inlined!");
            }
            fn inline_flattened(_: &Config) -> String {
                panic!("tuple cannot be flattened")
            }
            fn decl(_: &Config) -> String {
                panic!("tuple cannot be declared")
            }
            fn decl_concrete(_: &Config) -> String {
                panic!("tuple cannot be declared")
            }
            fn visit_generics(v: &mut impl TypeVisitor)
            where
                Self: 'static
            {
                $(
                    v.visit::<$i>();
                    <$i as crate::Flow>::visit_generics(v);
                )*
            }
        }
    };
    ( $i2:ident $(, $i:ident)* ) => {
        impl_flow_tuples!(impl $i2 $(, $i)* );
        impl_flow_tuples!($($i),*);
    };
    () => {};
}

impl_flow_tuples!(A, B, C, D, E, F, G, H, I, J, K, L);

// serde_json::Value → mixed
#[cfg(feature = "serde-json-impl")]
impl Flow for serde_json::Value {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;
    fn name(_: &Config) -> String {
        "mixed".to_owned()
    }
    fn inline(_: &Config) -> String {
        "mixed".to_owned()
    }
}

// PathBuf / &Path → string
impl Flow for std::path::PathBuf {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;
    fn name(_: &Config) -> String {
        "string".to_owned()
    }
    fn inline(_: &Config) -> String {
        "string".to_owned()
    }
}

impl Flow for std::path::Path {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;
    fn name(_: &Config) -> String {
        "string".to_owned()
    }
    fn inline(_: &Config) -> String {
        "string".to_owned()
    }
}

// Cow<'_, T> → T
impl_wrapper!(impl<'a, T: Flow + ToOwned + ?Sized> Flow for std::borrow::Cow<'a, T>);

// Result<T, E> → { ok: T } | { err: E }
impl<T: Flow, E: Flow> Flow for Result<T, E> {
    type WithoutGenerics = Result<Dummy, Dummy>;
    type OptionInnerType = Self;

    fn name(cfg: &Config) -> String {
        format!(
            "{{| +ok: {} |}} | {{| +err: {} |}}",
            T::name(cfg),
            E::name(cfg)
        )
    }
    fn inline(cfg: &Config) -> String {
        format!(
            "{{| +ok: {} |}} | {{| +err: {} |}}",
            T::inline(cfg),
            E::inline(cfg)
        )
    }
    fn visit_dependencies(v: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
        <T as crate::Flow>::visit_dependencies(v);
        <E as crate::Flow>::visit_dependencies(v);
    }
    fn visit_generics(v: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
        <T as crate::Flow>::visit_generics(v);
        v.visit::<T>();
        <E as crate::Flow>::visit_generics(v);
        v.visit::<E>();
    }
}

// Fixed-size arrays [T; N] → $ReadOnlyArray<T> (or tuple if small)
impl<T: Flow, const N: usize> Flow for [T; N] {
    type WithoutGenerics = [Dummy; N];
    type OptionInnerType = Self;

    fn name(cfg: &Config) -> String {
        if N > cfg.array_tuple_limit() {
            return <Vec<T> as crate::Flow>::name(cfg);
        }
        format!(
            "[{}]",
            (0..N)
                .map(|_| T::name(cfg))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
    fn inline(cfg: &Config) -> String {
        if N > cfg.array_tuple_limit() {
            return <Vec<T> as crate::Flow>::inline(cfg);
        }
        format!(
            "[{}]",
            (0..N)
                .map(|_| T::inline(cfg))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
    fn visit_dependencies(v: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
        <T as crate::Flow>::visit_dependencies(v);
    }
    fn visit_generics(v: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
        <T as crate::Flow>::visit_generics(v);
        v.visit::<T>();
    }
}

// Arc<T> → T
impl_wrapper!(impl<T: Flow + ?Sized> Flow for std::sync::Arc<T>);

// Rc<T> → T
impl_wrapper!(impl<T: Flow + ?Sized> Flow for std::rc::Rc<T>);

// Cell<T> → T
impl_wrapper!(impl<T: Flow> Flow for std::cell::Cell<T>);

// RefCell<T> → T
impl_wrapper!(impl<T: Flow> Flow for std::cell::RefCell<T>);
