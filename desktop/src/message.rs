use crate::{
    icon,
    state::{self, ResourceId},
};
use leptos::prelude::*;
use leptos_icons::Icon;

#[component]
pub fn Messages() -> impl IntoView {
    let state = expect_context::<state::State>();

    view! {
        <div>
            <For each=state.messages.read_only() key=|message| message.id().clone() let:message>
                <Message message />
            </For>
        </div>
    }
}

#[component]
pub fn Message(message: Message) -> impl IntoView {
    view! {
        <div>
            <div class="flex">
                <div class="grow">{message.title}</div>
                <div>
                    <button type="button" class="pointer-cursor">
                        <Icon icon=icon::Close />
                    </button>
                </div>
            </div>
            {message.body.map(|body| view! { <div>{body}</div> })}
        </div>
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    id: ResourceId,
    kind: Kind,
    title: String,
    body: Option<String>,
}

impl Message {
    pub fn error(title: impl Into<String>) -> Self {
        Self {
            id: ResourceId::new(),
            kind: Kind::Error,
            title: title.into(),
            body: None,
        }
    }

    pub fn error_with_body(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            id: ResourceId::new(),
            kind: Kind::Error,
            title: title.into(),
            body: Some(body.into()),
        }
    }

    pub fn id(&self) -> &ResourceId {
        &self.id
    }
}

#[derive(Debug, Clone, Copy)]
enum Kind {
    Success,
    Info,
    Warning,
    Error,
}
