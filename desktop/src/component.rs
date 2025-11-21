//! Generic components.
use leptos::prelude::*;

#[component]
pub fn ResizablePane(
    #[prop(optional, into)] class: String,
    #[prop(optional, default = 200)] width: usize,
    children: Children,
) -> impl IntoView {
    let width = format!("{width}px");
    let wrapper_class = format!("grow {class}");
    view! {
        <div class="flex">
            <div class="h-full w-[2px] border-l-4 border-transparent hover:border-primary-600 cursor-ew-resize"></div>
            <div style:width=width class=wrapper_class>
                {children()}
            </div>
        </div>
    }
}
