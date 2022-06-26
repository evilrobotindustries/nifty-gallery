use bulma::toast::Animate;
use bulma::{toast, toast::Position};

pub type Color = bulma::toast::Color;

pub(crate) fn notify(message: String, color: Option<Color>) {
    notify_extra_classes(message, color, None)
}

pub(crate) fn notify_extra_classes(
    message: String,
    color: Option<Color>,
    extra_classes: Option<String>,
) {
    toast::toast(
        message,
        color,
        Some(5000),
        Some(Position::BottomRight),
        None,
        Some(true),
        None,
        None,
        Some(Animate {
            in_: "flipInY".to_string(),
            out: "flipOutY".to_string(),
        }),
        extra_classes,
    );
}
