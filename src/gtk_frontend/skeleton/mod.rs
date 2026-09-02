mod css;
mod views;

pub(crate) use css::install_css;
#[allow(unused_imports)]
pub(crate) use views::detail;
pub(crate) use views::{downloads, home, resources, settings};
