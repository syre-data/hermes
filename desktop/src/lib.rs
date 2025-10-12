mod app;
mod component;
mod explorer;
mod formula;
mod message;
mod state;
mod workbook;

pub use app::App;

const LEVEL_PAD: usize = 6;
const LEVEL_PAD_UNIT: &str = "px";

/// For Tailwind to include classes
/// they must appear as string literals in at least one place.
/// This array is used to include them when needed.
static _TAILWIND_CLASSES: &'static [&'static str] = &[
    "group-hover/level-0:border-secondary-100",
    "group-hover/level-1:border-secondary-100",
    "group-hover/level-2:border-secondary-100",
    "group-hover/level-3:border-secondary-100",
    "group-hover/level-4:border-secondary-100",
    "group-hover/level-5:border-secondary-100",
    "group-hover/level-6:border-secondary-100",
    "group-hover/level-7:border-secondary-100",
    "group-hover/level-8:border-secondary-100",
    "group-hover/level-9:border-secondary-100",
    "group-hover/level-10:border-secondary-100",
    "group-hover/level-11:border-secondary-100",
    "group-hover/level-12:border-secondary-100",
    "group-hover/level-13:border-secondary-100",
    "group-hover/level-14:border-secondary-100",
    "group-hover/level-15:border-secondary-100",
    "group-hover/level-16:border-secondary-100",
    "group-hover/level-17:border-secondary-100",
    "group-hover/level-18:border-secondary-100",
    "group-hover/level-19:border-secondary-100",
    "dark:group-hover/level-0:border-secondary-600",
    "dark:group-hover/level-1:border-secondary-600",
    "dark:group-hover/level-2:border-secondary-600",
    "dark:group-hover/level-2:border-secondary-600",
    "dark:group-hover/level-3:border-secondary-600",
    "dark:group-hover/level-4:border-secondary-600",
    "dark:group-hover/level-5:border-secondary-600",
    "dark:group-hover/level-6:border-secondary-600",
    "dark:group-hover/level-7:border-secondary-600",
    "dark:group-hover/level-8:border-secondary-600",
    "dark:group-hover/level-9:border-secondary-600",
    "dark:group-hover/level-10:border-secondary-600",
    "dark:group-hover/level-11:border-secondary-600",
    "dark:group-hover/level-12:border-secondary-600",
    "dark:group-hover/level-12:border-secondary-600",
    "dark:group-hover/level-13:border-secondary-600",
    "dark:group-hover/level-14:border-secondary-600",
    "dark:group-hover/level-15:border-secondary-600",
    "dark:group-hover/level-16:border-secondary-600",
    "dark:group-hover/level-17:border-secondary-600",
    "dark:group-hover/level-18:border-secondary-600",
    "dark:group-hover/level-19:border-secondary-600",
];

mod icon {
    pub use icondata::{
        AiCloseOutlined as Close, AiLoading3QuartersOutlined as LoadingSpinner,
        AiMinusOutlined as Remove, AiPlusOutlined as Add, FaEqualsSolid as Equal,
        MdiFunction as Function,
    };
}

mod types {
    /// Enum for different mouse buttons
    /// for use with `MouseEvent::button`.
    /// See https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/button#value.
    #[derive(Clone, Copy, Debug)]
    pub enum MouseButton {
        Primary = 0,
        // Auxillary = 1,
        // Secondary = 2,
        // Fourth = 3,
        // Fifth = 4,
    }

    impl PartialEq<i16> for MouseButton {
        fn eq(&self, other: &i16) -> bool {
            (*self as i16).eq(other)
        }
    }

    impl PartialEq<MouseButton> for i16 {
        fn eq(&self, other: &MouseButton) -> bool {
            other.eq(self)
        }
    }
}
