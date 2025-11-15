use std::{ffi::OsString, path::PathBuf};

#[derive(Debug, derive_more::From)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Event {
    File(File),
    Folder(Folder),

    /// Could not determine if the event affects a file, folder, or other resource.
    Any(Any),
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum File {
    Created(PathBuf),
    Removed(PathBuf),
    Renamed {
        from: PathBuf,

        #[serde(with = "serialize_os_string")]
        to: OsString,
    },
    Moved {
        from: PathBuf,
        to: PathBuf,
    },
    Modified(PathBuf),
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Folder {
    Created(PathBuf),
    Removed(PathBuf),
    Renamed {
        from: PathBuf,
        #[serde(with = "serialize_os_string")]
        to: OsString,
    },
    Moved {
        from: PathBuf,
        to: PathBuf,
    },
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Any {
    Removed(PathBuf),
}

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
