// Copyright © 2025 Nipun Kumar

use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq, Debug)]
pub struct SlideoutProps {
    pub open: Signal<bool>,
    pub children: Element,
}

#[component]
pub fn Slideout(mut props: SlideoutProps) -> Element {
    if !*props.open.read() {
        return rsx! {};
    }

    rsx! {
        // Backdrop overlay that closes the slideout when clicked
        div {
            style: "
                position: fixed;
                top: 0;
                left: 0;
                width: 100%;
                height: 100%;
                background: rgba(0, 0, 0, 0.3);
                z-index: 998;
            ",
            onclick: move |_| {
                props.open.set(false);
            },

            // Slideout panel
            div {
                style: "
                    position: fixed;
                    top: 0;
                    right: 0;
                    max-width: 80%;
                    height: 100%;
                    background: #fff;
                    z-index: 999;
                ",
                onclick: move |e: Event<MouseData>| {
                    e.stop_propagation();
                },
                {props.children}
            }
        }
    }
}
