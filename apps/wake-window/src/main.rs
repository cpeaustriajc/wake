//! Wake desktop window: an empty GPUI window, nothing more yet.

use gpui::{App, Context, IntoElement, Render, Window, WindowOptions, div, prelude::*};

struct Root;

impl Render for Root {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        div().child(wake::name())
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        cx.open_window(WindowOptions::default(), |_window, cx| cx.new(|_| Root))
            .expect("the window should open");
        cx.activate(true);
    });
}
