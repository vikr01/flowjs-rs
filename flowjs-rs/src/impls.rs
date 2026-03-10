//! Built-in Flow trait implementations for Rust standard types.

use crate::{Config, Flow};

macro_rules! impl_flow_primitive {
    ($rust_ty:ty, $flow_ty:expr) => {
        impl Flow for $rust_ty {
            fn name(_: &Config) -> String {
                $flow_ty.to_owned()
            }
            fn inline(_: &Config) -> String {
                $flow_ty.to_owned()
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
    fn name(cfg: &Config) -> String {
        format!("?{}", T::name(cfg))
    }
    fn inline(cfg: &Config) -> String {
        format!("?{}", T::inline(cfg))
    }
}

// Vec<T> → $ReadOnlyArray<T>
impl<T: Flow> Flow for Vec<T> {
    fn name(cfg: &Config) -> String {
        format!("$ReadOnlyArray<{}>", T::name(cfg))
    }
    fn inline(cfg: &Config) -> String {
        format!("$ReadOnlyArray<{}>", T::inline(cfg))
    }
}

// &[T] → $ReadOnlyArray<T>
impl<T: Flow> Flow for [T] {
    fn name(cfg: &Config) -> String {
        format!("$ReadOnlyArray<{}>", T::name(cfg))
    }
    fn inline(cfg: &Config) -> String {
        format!("$ReadOnlyArray<{}>", T::inline(cfg))
    }
}

// Box<T> → T
impl<T: Flow + ?Sized> Flow for Box<T> {
    fn name(cfg: &Config) -> String {
        T::name(cfg)
    }
    fn inline(cfg: &Config) -> String {
        T::inline(cfg)
    }
}

// &T → T
impl<'a, T: Flow + ?Sized> Flow for &'a T {
    fn name(cfg: &Config) -> String {
        T::name(cfg)
    }
    fn inline(cfg: &Config) -> String {
        T::inline(cfg)
    }
}

// std::collections::HashMap → { [key: K]: V }
impl<K: Flow, V: Flow> Flow for std::collections::HashMap<K, V> {
    fn name(cfg: &Config) -> String {
        format!("{{ [key: {}]: {} }}", K::name(cfg), V::name(cfg))
    }
    fn inline(cfg: &Config) -> String {
        format!("{{ [key: {}]: {} }}", K::inline(cfg), V::inline(cfg))
    }
}

// std::collections::BTreeMap → { [key: K]: V }
impl<K: Flow, V: Flow> Flow for std::collections::BTreeMap<K, V> {
    fn name(cfg: &Config) -> String {
        format!("{{ [key: {}]: {} }}", K::name(cfg), V::name(cfg))
    }
    fn inline(cfg: &Config) -> String {
        format!("{{ [key: {}]: {} }}", K::inline(cfg), V::inline(cfg))
    }
}

// HashSet → $ReadOnlyArray
impl<T: Flow> Flow for std::collections::HashSet<T> {
    fn name(cfg: &Config) -> String {
        format!("$ReadOnlyArray<{}>", T::name(cfg))
    }
    fn inline(cfg: &Config) -> String {
        format!("$ReadOnlyArray<{}>", T::inline(cfg))
    }
}

// BTreeSet → $ReadOnlyArray
impl<T: Flow> Flow for std::collections::BTreeSet<T> {
    fn name(cfg: &Config) -> String {
        format!("$ReadOnlyArray<{}>", T::name(cfg))
    }
    fn inline(cfg: &Config) -> String {
        format!("$ReadOnlyArray<{}>", T::inline(cfg))
    }
}

// Tuples
macro_rules! impl_flow_tuple {
    ($($t:ident),+) => {
        impl<$($t: Flow),+> Flow for ($($t,)+) {
            fn name(cfg: &Config) -> String {
                let parts: Vec<String> = vec![$($t::name(cfg)),+];
                format!("[{}]", parts.join(", "))
            }
            fn inline(cfg: &Config) -> String {
                let parts: Vec<String> = vec![$($t::inline(cfg)),+];
                format!("[{}]", parts.join(", "))
            }
        }
    };
}

impl_flow_tuple!(A);
impl_flow_tuple!(A, B);
impl_flow_tuple!(A, B, C);
impl_flow_tuple!(A, B, C, D);
impl_flow_tuple!(A, B, C, D, E);
impl_flow_tuple!(A, B, C, D, E, F);
impl_flow_tuple!(A, B, C, D, E, F, G);
impl_flow_tuple!(A, B, C, D, E, F, G, H);
impl_flow_tuple!(A, B, C, D, E, F, G, H, I);
impl_flow_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_flow_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_flow_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);

// serde_json::Value → mixed
#[cfg(feature = "serde-json-impl")]
impl Flow for serde_json::Value {
    fn name(_: &Config) -> String {
        "mixed".to_owned()
    }
    fn inline(_: &Config) -> String {
        "mixed".to_owned()
    }
}

// PathBuf / &Path → string
impl Flow for std::path::PathBuf {
    fn name(_: &Config) -> String {
        "string".to_owned()
    }
    fn inline(_: &Config) -> String {
        "string".to_owned()
    }
}

impl Flow for std::path::Path {
    fn name(_: &Config) -> String {
        "string".to_owned()
    }
    fn inline(_: &Config) -> String {
        "string".to_owned()
    }
}

// Cow<'_, T> → T
impl<'a, T: Flow + ToOwned + ?Sized> Flow for std::borrow::Cow<'a, T> {
    fn name(cfg: &Config) -> String {
        T::name(cfg)
    }
    fn inline(cfg: &Config) -> String {
        T::inline(cfg)
    }
}

// Result<T, E> → { ok: T } | { err: E }
impl<T: Flow, E: Flow> Flow for Result<T, E> {
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
}

// Fixed-size arrays [T; N] → $ReadOnlyArray<T>
impl<T: Flow, const N: usize> Flow for [T; N] {
    fn name(cfg: &Config) -> String {
        format!("$ReadOnlyArray<{}>", T::name(cfg))
    }
    fn inline(cfg: &Config) -> String {
        format!("$ReadOnlyArray<{}>", T::inline(cfg))
    }
}
