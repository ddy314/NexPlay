mod bootstrap;
mod components;
mod events;
mod images;
mod oauth;
mod pages;
mod player;
mod prelude;
mod runtime;
mod shell;
mod skeleton;
mod state;

pub fn run() -> crate::error::AppResult<()> {
    bootstrap::run()
}
