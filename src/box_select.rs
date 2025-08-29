use dioxus::prelude::*;

#[component]
pub fn BoxSelect(
    value: Option<String>,
    options: Vec<String>,
    on_select: Callback<Option<String>, ()>,
) -> Element {
    let selected_none_class = if value.is_none() { "selected" } else { "" };
    rsx! {
        div { class: "box-select", style: "",
            div {
                class: "option {selected_none_class}",
                onclick: move |_e| { on_select(None) },
                "-- Select One --"
            }
            {
                options
                    .into_iter()
                    .map(move |o| {
                        let selected = if let Some(v) = &value { &o == v } else { false };
                        let selected_class = if selected { "selected" } else { "" };
                        rsx! {
                            div {
                                class: "option {selected_class}",
                                onclick: move |_e| { on_select(Some(o.clone())) },
                                "{o}"
                            }
                        }
                    })
            }
        }
    }
}
