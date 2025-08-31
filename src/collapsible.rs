use dioxus::prelude::*;

#[component]
pub fn Collapsible(c: bool, children: Element) -> Element {
    let mut collapsed = use_signal(|| c);
    rsx! {
        button {
            style: "
            float: inline-start;
            position: relative;
            top: -1em;
            height: 2em;
            width: 2em;
            ",
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
