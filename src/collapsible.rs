use dioxus::prelude::*;

#[component]
pub fn Collapsible(c: bool, children: Element) -> Element {
    let mut collapsed = use_signal(|| c);
    rsx! {
            button {
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
