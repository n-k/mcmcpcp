// Copyright © 2025 Nipun Kumar

use dioxus::prelude::*;

// const EXPAND_ICON: Asset = asset!("/assets/expand.png");
// const COLLAPSE_ICON: Asset = asset!("/assets/collapse.png");

#[component]
pub fn Collapsible(c: bool, children: Element) -> Element {
    let mut collapsed = use_signal(|| c);
    rsx! {
        button {
            class: "delete-group-btn",
            style: "
            position: relative;
            top: 5px;
            left: 5px;
            background: rgba(255, 255, 255, 0.2);
            color: white;
            border: none;
            border-radius: 50%;
            width: 20px;
            height: 20px;
            cursor: pointer;
            font-size: 14px;
            display: flex;
            align-items: center;
            justify-content: center;
            opacity: 0.7;
            z-index: 10;
            ",
            onclick: move |_e| {
                collapsed.toggle();
            },
            if collapsed() {
                // img { src: EXPAND_ICON }
                ">"
            } else {
                // img { src: COLLAPSE_ICON }
                "<"
            }
        }
        if !collapsed() {
            {children}
        }
    }
}
