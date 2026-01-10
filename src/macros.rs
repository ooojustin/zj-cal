pub(crate) const CONTEXT_SOURCE_KEY: &str = "source";

macro_rules! log {
    ($($arg:tt)*) => {
        eprintln!("[zj-cal] {}", format!($($arg)*))
    };
}

macro_rules! ctx_insert_field {
    ($map:expr, $field:ident, Option<$inner:ty>) => {
        if let Some(value) = $field {
            $map.insert(stringify!($field).to_string(), value.to_string());
        }
    };
    ($map:expr, $field:ident, std::option::Option<$inner:ty>) => {
        if let Some(value) = $field {
            $map.insert(stringify!($field).to_string(), value.to_string());
        }
    };
    ($map:expr, $field:ident, $ty:ty) => {
        $map.insert(stringify!($field).to_string(), $field.to_string());
    };
}

macro_rules! ctx_parse_field {
    ($map:expr, $field:ident, Option<$inner:ty>) => {{
        let key = stringify!($field);
        match $map.get(key) {
            Some(raw) => raw
                .parse::<$inner>()
                .map(Some)
                .map_err(|_| format!("invalid '{}' value '{}'", key, raw)),
            None => Ok(None),
        }
    }};
    ($map:expr, $field:ident, std::option::Option<$inner:ty>) => {{
        let key = stringify!($field);
        match $map.get(key) {
            Some(raw) => raw
                .parse::<$inner>()
                .map(Some)
                .map_err(|_| format!("invalid '{}' value '{}'", key, raw)),
            None => Ok(None),
        }
    }};
    ($map:expr, $field:ident, $ty:ty) => {{
        let key = stringify!($field);
        match $map.get(key) {
            Some(raw) => raw
                .parse::<$ty>()
                .map_err(|_| format!("invalid '{}' value '{}'", key, raw)),
            None => Err(format!("missing '{}' field", key)),
        }
    }};
}

macro_rules! ctx_from_map_value {
    ($map:expr, $variant:ident) => {
        Ok(Ctx::$variant)
    };
    ($map:expr, $variant:ident, { $($field:ident : $ty:ty),+ $(,)? }) => {{
        Ok(Ctx::$variant {
            $(
                $field: ctx_parse_field!($map, $field, $ty)?,
            )+
        })
    }};
}

macro_rules! ctx_into_map_value {
    ($map:expr, $source:literal) => {
        $map.insert(
            $crate::macros::CONTEXT_SOURCE_KEY.to_string(),
            $source.to_string(),
        );
    };
    ($map:expr, $source:literal, { $($field:ident : $ty:ty),+ $(,)? }) => {
        $map.insert(
            $crate::macros::CONTEXT_SOURCE_KEY.to_string(),
            $source.to_string(),
        );
        $(
            ctx_insert_field!($map, $field, $ty);
        )+
    };
}

macro_rules! define_ctx {
    ($(
        $variant:ident $( { $($field:ident : $ty:ty),+ $(,)? } )? => $source:literal
    ),+ $(,)?) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum Ctx {
            $( $variant $( { $($field: $ty),+ } )?, )+
        }

        impl Ctx {
            pub fn into_map(self) -> ::std::collections::BTreeMap<String, String> {
                let mut map = ::std::collections::BTreeMap::new();
                match self {
                    $(
                        Ctx::$variant $( { $($field),+ } )? => {
                            ctx_into_map_value!(map, $source $(, { $($field : $ty),+ } )?);
                        }
                    ),+
                }
                map
            }

            pub fn from_map(
                map: &::std::collections::BTreeMap<String, String>,
            ) -> Result<Self, String> {
                let source = map
                    .get($crate::macros::CONTEXT_SOURCE_KEY)
                    .ok_or_else(|| {
                        format!(
                            "missing '{}' key",
                            $crate::macros::CONTEXT_SOURCE_KEY
                        )
                    })?;
                match source.as_str() {
                    $( $source => ctx_from_map_value!(map, $variant $(, { $($field : $ty),+ } )? ), )+
                    _ => Err(format!(
                        "unknown '{}' value '{}'",
                        $crate::macros::CONTEXT_SOURCE_KEY,
                        source
                    )),
                }
            }
        }
    };
}
