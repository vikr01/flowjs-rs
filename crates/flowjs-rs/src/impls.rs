//! Built-in Flow trait implementations for Rust standard types.

use crate::{flow_type, Config, Dummy, Flow, TypeVisitor};

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
impl_flow_primitive!(bool, flow_type::BOOLEAN);
impl_flow_primitive!(i8, flow_type::NUMBER);
impl_flow_primitive!(i16, flow_type::NUMBER);
impl_flow_primitive!(i32, flow_type::NUMBER);
impl_flow_primitive!(u8, flow_type::NUMBER);
impl_flow_primitive!(u16, flow_type::NUMBER);
impl_flow_primitive!(u32, flow_type::NUMBER);
// Large integers — configurable via Config::large_int() (default: bigint).
// These cannot use impl_flow_primitive! because the type is dynamic.
macro_rules! impl_flow_large_int {
    ($($rust_ty:ty),+) => {$(
        impl Flow for $rust_ty {
            type WithoutGenerics = Self;
            type OptionInnerType = Self;
            fn name(cfg: &Config) -> String {
                cfg.large_int().to_owned()
            }
            fn inline(cfg: &Config) -> String {
                cfg.large_int().to_owned()
            }
        }
    )+};
}

impl_flow_large_int!(i64, u64, i128, u128);
impl_flow_primitive!(f32, flow_type::NUMBER);
impl_flow_primitive!(f64, flow_type::NUMBER);
impl_flow_primitive!(char, flow_type::STRING);
impl_flow_primitive!(String, flow_type::STRING);
impl_flow_primitive!(str, flow_type::STRING);
impl_flow_primitive!((), flow_type::NULL);

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
        flow_type::READ_ONLY_ARRAY.to_owned()
    }
    fn name(cfg: &Config) -> String {
        format!("{}<{}>", flow_type::READ_ONLY_ARRAY, T::name(cfg))
    }
    fn inline(cfg: &Config) -> String {
        format!("{}<{}>", flow_type::READ_ONLY_ARRAY, T::inline(cfg))
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

// &mut T → T
impl_wrapper!(impl<'a, T: Flow + ?Sized> Flow for &'a mut T);

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

// VecDeque → $ReadOnlyArray
impl_shadow!(as Vec<T>: impl<T: Flow> Flow for std::collections::VecDeque<T>);

// LinkedList → $ReadOnlyArray
impl_shadow!(as Vec<T>: impl<T: Flow> Flow for std::collections::LinkedList<T>);

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
            fn inline(cfg: &Config) -> String {
                let parts: Vec<String> = vec![$($i::inline(cfg)),*];
                format!("[{}]", parts.join(", "))
            }
            fn inline_flattened(cfg: &Config) -> String {
                format!("({})", Self::inline(cfg))
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

impl_flow_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

// serde_json::Value → mixed (declared as `type JsonValue = mixed` for import parity with ts-rs)
#[cfg(feature = "serde-json-impl")]
impl Flow for serde_json::Value {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;
    fn name(_: &Config) -> String {
        "JsonValue".to_owned()
    }
    fn inline(_: &Config) -> String {
        flow_type::MIXED.to_owned()
    }
    fn decl(_: &Config) -> String {
        format!("type JsonValue = {};", flow_type::MIXED)
    }
    fn decl_concrete(_: &Config) -> String {
        format!("type JsonValue = {};", flow_type::MIXED)
    }
    fn output_path() -> Option<std::path::PathBuf> {
        Some(std::path::PathBuf::from("JsonValue"))
    }
}

// PathBuf / &Path → string
impl Flow for std::path::PathBuf {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;
    fn name(_: &Config) -> String {
        flow_type::STRING.to_owned()
    }
    fn inline(_: &Config) -> String {
        flow_type::STRING.to_owned()
    }
}

impl Flow for std::path::Path {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;
    fn name(_: &Config) -> String {
        flow_type::STRING.to_owned()
    }
    fn inline(_: &Config) -> String {
        flow_type::STRING.to_owned()
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
            "{{| Ok: {} |}} | {{| Err: {} |}}",
            T::name(cfg),
            E::name(cfg)
        )
    }
    fn inline(cfg: &Config) -> String {
        format!(
            "{{| Ok: {} |}} | {{| Err: {} |}}",
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
            (0..N).map(|_| T::name(cfg)).collect::<Vec<_>>().join(", ")
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

// Mutex<T> → T
impl_wrapper!(impl<T: Flow> Flow for std::sync::Mutex<T>);

// RwLock<T> → T
impl_wrapper!(impl<T: Flow> Flow for std::sync::RwLock<T>);

// usize / isize
impl_flow_primitive!(usize, flow_type::NUMBER);
impl_flow_primitive!(isize, flow_type::NUMBER);

// Infallible → empty (bottom type, never inhabited)
impl_flow_primitive!(std::convert::Infallible, flow_type::EMPTY);

// Wrapping<T> → T (serializes as inner type)
impl_wrapper!(impl<T: Flow> Flow for std::num::Wrapping<T>);

// Saturating<T> → T (serializes as inner type)
impl_wrapper!(impl<T: Flow> Flow for std::num::Saturating<T>);

// NonZero* types → number
impl_shadow!(as u8: impl Flow for std::num::NonZeroU8);
impl_shadow!(as u16: impl Flow for std::num::NonZeroU16);
impl_shadow!(as u32: impl Flow for std::num::NonZeroU32);
impl_shadow!(as u64: impl Flow for std::num::NonZeroU64);
impl_shadow!(as u128: impl Flow for std::num::NonZeroU128);
impl_shadow!(as usize: impl Flow for std::num::NonZeroUsize);
impl_shadow!(as i8: impl Flow for std::num::NonZeroI8);
impl_shadow!(as i16: impl Flow for std::num::NonZeroI16);
impl_shadow!(as i32: impl Flow for std::num::NonZeroI32);
impl_shadow!(as i64: impl Flow for std::num::NonZeroI64);
impl_shadow!(as i128: impl Flow for std::num::NonZeroI128);
impl_shadow!(as isize: impl Flow for std::num::NonZeroIsize);

// PhantomData<T> → void
impl<T: ?Sized> Flow for std::marker::PhantomData<T> {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;
    fn name(_: &Config) -> String {
        flow_type::VOID.to_owned()
    }
    fn inline(_: &Config) -> String {
        flow_type::VOID.to_owned()
    }
}

// Range<T> → { +start: T, +end: T }
impl<T: Flow> Flow for std::ops::Range<T> {
    type WithoutGenerics = std::ops::Range<Dummy>;
    type OptionInnerType = Self;

    fn ident(_: &Config) -> String {
        panic!()
    }
    fn name(cfg: &Config) -> String {
        format!("{{ +start: {}, +end: {} }}", T::name(cfg), T::name(cfg))
    }
    fn inline(cfg: &Config) -> String {
        format!("{{ +start: {}, +end: {} }}", T::inline(cfg), T::inline(cfg))
    }
    fn inline_flattened(cfg: &Config) -> String {
        format!("({})", Self::inline(cfg))
    }
    fn visit_dependencies(v: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
        T::visit_dependencies(v);
    }
    fn visit_generics(v: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
        T::visit_generics(v);
        v.visit::<T>();
    }
}

// RangeInclusive<T> → { +start: T, +end: T }
impl_shadow!(as std::ops::Range<T>: impl<T: Flow> Flow for std::ops::RangeInclusive<T>);

// Duration → { +secs: number, +nanos: number }
impl Flow for std::time::Duration {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;
    fn name(_: &Config) -> String {
        format!(
            "{{| +secs: {}, +nanos: {} |}}",
            flow_type::NUMBER,
            flow_type::NUMBER
        )
    }
    fn inline(cfg: &Config) -> String {
        Self::name(cfg)
    }
}

// SystemTime → string (ISO 8601 serialization)
impl_flow_primitive!(std::time::SystemTime, flow_type::STRING);

// Network address types → string
impl_flow_primitive!(std::net::IpAddr, flow_type::STRING);
impl_flow_primitive!(std::net::Ipv4Addr, flow_type::STRING);
impl_flow_primitive!(std::net::Ipv6Addr, flow_type::STRING);
impl_flow_primitive!(std::net::SocketAddr, flow_type::STRING);
impl_flow_primitive!(std::net::SocketAddrV4, flow_type::STRING);
impl_flow_primitive!(std::net::SocketAddrV6, flow_type::STRING);

// chrono types → string
#[cfg(feature = "chrono-impl")]
impl_flow_primitive!(chrono::NaiveDate, flow_type::STRING);
#[cfg(feature = "chrono-impl")]
impl_flow_primitive!(chrono::NaiveTime, flow_type::STRING);
#[cfg(feature = "chrono-impl")]
impl_flow_primitive!(chrono::NaiveDateTime, flow_type::STRING);
#[cfg(feature = "chrono-impl")]
impl<T: chrono::TimeZone> Flow for chrono::DateTime<T> {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;
    fn name(_: &Config) -> String {
        flow_type::STRING.to_owned()
    }
    fn inline(_: &Config) -> String {
        flow_type::STRING.to_owned()
    }
}
#[cfg(feature = "chrono-impl")]
impl_flow_primitive!(chrono::Duration, flow_type::STRING);

// uuid::Uuid → string
#[cfg(feature = "uuid-impl")]
impl_flow_primitive!(uuid::Uuid, flow_type::STRING);

// url::Url → string
#[cfg(feature = "url-impl")]
impl_flow_primitive!(url::Url, flow_type::STRING);

// fn pointers → Flow function types: (arg0: A, arg1: B) => R
macro_rules! impl_flow_fn {
    // Base case: fn() -> R (no arguments)
    (impl fn() -> R) => {
        impl<R: Flow> Flow for fn() -> R {
            type WithoutGenerics = Self;
            type OptionInnerType = Self;
            fn name(cfg: &Config) -> String {
                format!("() => {}", R::name(cfg))
            }
            fn inline(cfg: &Config) -> String {
                format!("() => {}", R::inline(cfg))
            }
        }
    };
    // Recursive case: fn(A, B, ...) -> R
    (impl fn($($i:tt: $T:ident),+) -> R) => {
        impl<$($T: Flow,)+ R: Flow> Flow for fn($($T),+) -> R {
            type WithoutGenerics = Self;
            type OptionInnerType = Self;
            fn name(cfg: &Config) -> String {
                let params = vec![$(format!("arg{}: {}", $i, $T::name(cfg))),+];
                format!("({}) => {}", params.join(", "), R::name(cfg))
            }
            fn inline(cfg: &Config) -> String {
                let params = vec![$(format!("arg{}: {}", $i, $T::inline(cfg))),+];
                format!("({}) => {}", params.join(", "), R::inline(cfg))
            }
            fn visit_generics(v: &mut impl TypeVisitor)
            where
                Self: 'static,
            {
                $(v.visit::<$T>();)+
                v.visit::<R>();
            }
        }
    };
}

impl_flow_fn!(impl fn() -> R);
impl_flow_fn!(impl fn(0: A) -> R);
impl_flow_fn!(impl fn(0: A, 1: B) -> R);
impl_flow_fn!(impl fn(0: A, 1: B, 2: C) -> R);
impl_flow_fn!(impl fn(0: A, 1: B, 2: C, 3: D) -> R);
impl_flow_fn!(impl fn(0: A, 1: B, 2: C, 3: D, 4: E) -> R);
impl_flow_fn!(impl fn(0: A, 1: B, 2: C, 3: D, 4: E, 5: F) -> R);
impl_flow_fn!(impl fn(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G) -> R);
impl_flow_fn!(impl fn(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H) -> R);
impl_flow_fn!(impl fn(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I) -> R);
impl_flow_fn!(impl fn(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J) -> R);
impl_flow_fn!(impl fn(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K) -> R);
impl_flow_fn!(impl fn(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K, 11: L) -> R);
