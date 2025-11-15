pub mod data;
pub mod event;
pub mod formula;
pub mod fs;

pub mod utils {
    pub mod serialize_os_string {
        use std::{
            ffi::{OsStr, OsString},
            fmt,
        };

        pub fn serialize<T, S>(value: &T, ser: S) -> Result<S::Ok, S::Error>
        where
            T: AsRef<OsStr>,
            S: serde::Serializer,
        {
            let value = value.as_ref().to_string_lossy();
            ser.serialize_str(&value)
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<OsString, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_str(Visitor)
        }

        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = OsString;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("os string compatible value")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(OsString::from(v))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(OsString::from(v))
            }
        }
    }
}
