use dioxus::prelude::*;

#[component]
pub fn Collapsible(c: bool, children: Element) -> Element {
    let mut collapsed = use_signal(|| c);
    rsx! {
        button {
            style: "position: relative; top: -2.5em;",
            onclick: move |_e| {
                collapsed.toggle();
            },
            if collapsed() {
                "+"
            } else {
                "-"
            }
        }
        if !collapsed() {
            {children}
        }
    }
}
