use serde::Serialize;
use wasm_bindgen::prelude::*;

pub fn toast(
    message: String,
    color: Option<Color>,
    duration: Option<u32>,
    position: Option<Position>,
    dismissable: Option<bool>,
    pause_on_hover: Option<bool>,
    close_on_click: Option<bool>,
    opacity: Option<f32>,
    animate: Option<Animate>,
    extra_classes: Option<String>,
) {
    let options = Options {
        message,
        toast_type: color.as_ref().map(|c| c.as_str()),
        duration,
        position: position.as_ref().map(|p| p.as_str()),
        dismissible: dismissable,
        pause_on_hover,
        close_on_click,
        opacity,
        animate,
        extra_classes,
    };
    default::toast(JsValue::from_serde(&options).expect("could not serialise options"));
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Animate {
    #[serde(rename = "in")]
    pub in_: String,
    pub out: String,
}

pub enum Color {
    Primary,
    Link,
    Info,
    Success,
    Warning,
    Danger,
    Custom(String),
}

impl Color {
    fn as_str(&self) -> &str {
        match self {
            Color::Primary => "is-primary",
            Color::Link => "is-link",
            Color::Info => "is-info",
            Color::Success => "is-success",
            Color::Warning => "is-warning",
            Color::Danger => "is-danger",
            Color::Custom(class) => class.as_str(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Options<'a> {
    /// The actual message to be displayed.
    pub message: String,
    /// Essentially a Bulma's css class. It can be is-primary, is-link, is-info, is-success, is-warning, is-danger, or any other custom class. Default is a whitesmoke background with dark text
    #[serde(rename = "type")]
    pub toast_type: Option<&'a str>,
    ///  Duration of the notification in milliseconds. Default is 2000 milliseconds.
    pub duration: Option<u32>,
    /// Position where the notification will be shown. The default is top-right, so if you want it to be on the top-left just add top-left to this option. The available options are: top-left, top-center, top-right, center, bottom-left, bottom-center, and bottom-right.
    pub position: Option<&'a str>,
    /// Whether the notification will have a close button or not. Default is false.
    pub dismissible: Option<bool>,
    /// Pauses delay when hovering the notification. Default is false.
    pub pause_on_hover: Option<bool>,
    ///  Dismisses the notification when clicked. Default is true.
    pub close_on_click: Option<bool>,
    /// The notification's container opacity. Default is 1
    pub opacity: Option<f32>,
    /// Any animate.css animations to be used.
    pub animate: Option<Animate>,
    /// Adds classes for styling the toast notification.
    pub extra_classes: Option<String>,
}

pub enum Position {
    TopLeft,
    TopCenter,
    TopRight,
    Center,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl Position {
    fn as_str(&self) -> &str {
        match self {
            Position::TopLeft => "top-left",
            Position::TopCenter => "top-center",
            Position::TopRight => "top-right",
            Position::Center => "center",
            Position::BottomLeft => "bottom-left",
            Position::BottomCenter => "bottom-center",
            Position::BottomRight => "bottom-right",
        }
    }
}

#[wasm_bindgen(module = "/assets/bulma-toast.min.js")]
extern "C" {
    #[allow(non_camel_case_types)]
    type default;

    #[wasm_bindgen(static_method_of = default)]
    pub fn toast(options: JsValue);
}
