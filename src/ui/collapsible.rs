use dioxus::prelude::*;

const EXPAND_ICON: Asset = asset!("/assets/expand.svg");
const COLLAPSE_ICON: Asset = asset!("/assets/collapse.svg");

#[component]
pub fn Collapsible(c: bool, children: Element) -> Element {
    let mut collapsed = use_signal(|| c);
    rsx! {
        button {
            style: "
            float: inline-start;
            position: relative;
            top: -24px;
            height: 24px;
            width: 24px;
            ",
            onclick: move |_e| {
                collapsed.toggle();
            },
            if collapsed() {
                img { src: EXPAND_ICON }
            } else {
                img { src: COLLAPSE_ICON }
            }
        }
        if !collapsed() {
            {children}
        }
    }
}
