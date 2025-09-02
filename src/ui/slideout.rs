use dioxus::{logger::tracing::warn, prelude::*};

#[component]
pub fn Slideout(open: Signal<bool>, children: Element) -> Element {
    let transform = if open() { 
        "transform: translateX(0);"
    } else { 
        "transform: translateX(100%);"
    };
    rsx! {
        div {
            style: "
            position: fixed;
            top: 0;
            right: 0;
            width: 250px;
            height: 100%;
            background: #fff;
            box-shadow: -2px 0 6px rgba(0,0,0,.2);
            {transform}
            transition: transform 0.3s ease;
            ",
            {children}
        }
    }
}
