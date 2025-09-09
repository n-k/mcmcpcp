use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct SlideoutProps {
    pub open: Signal<bool>,
    pub children: Element,
}

#[component]
pub fn Slideout(mut props: SlideoutProps) -> Element {
    let transform = if *props.open.read() {
        "transform: translateX(0);"
    } else {
        "transform: translateX(100%);"
    };

    let visibility = if *props.open.read() {
        "visibility: visible; opacity: 1;"
    } else {
        "visibility: hidden; opacity: 0;"
    };

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
                {visibility}
                transition: opacity 0.3s ease, visibility 0.3s ease;
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
                    width: 500px;
                    height: 100%;
                    background: #fff;
                    box-shadow: -2px 0 6px rgba(0,0,0,.2);
                    {transform}
                    transition: transform 0.3s ease;
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
