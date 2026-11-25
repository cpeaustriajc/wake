//! The Wake fleet view on GPUI: five fixed agents, each with a headline, a phase and where it runs.

use gpui::{
    App, Bounds, Hsla, IntoElement, LineCap, PathBuilder, PathStyle, Pixels, Render, Rgba,
    StrokeOptions, TitlebarOptions, Window, WindowBounds, WindowOptions, canvas, div, point,
    prelude::*, px, rgb, size,
};

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Planning,
    Working,
    Review,
    Done,
}

impl Phase {
    const ALL: [Phase; 4] = [Phase::Planning, Phase::Working, Phase::Review, Phase::Done];

    fn label(self) -> &'static str {
        match self {
            Phase::Planning => "Planning",
            Phase::Working => "Working",
            Phase::Review => "Review",
            Phase::Done => "Done",
        }
    }

    fn step(self) -> u8 {
        let position = Self::ALL
            .iter()
            .position(|phase| *phase == self)
            .unwrap_or(0);
        u8::try_from(position).unwrap_or(0)
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Place {
    Local,
    Cloud,
}

struct Agent {
    name: &'static str,
    headline: &'static str,
    phase: Phase,
    place: Place,
}

const FLEET: [Agent; 5] = [
    Agent {
        name: "auth-refactor",
        headline: "Splitting token refresh out of the session module",
        phase: Phase::Working,
        place: Place::Local,
    },
    Agent {
        name: "billing-webhooks",
        headline: "Waiting for a choice between retry and dead-letter",
        phase: Phase::Review,
        place: Place::Local,
    },
    Agent {
        name: "migrate-db",
        headline: "Moving the events table to the new schema",
        phase: Phase::Working,
        place: Place::Cloud,
    },
    Agent {
        name: "docs-sweep",
        headline: "Fixed 41 broken links across the guides",
        phase: Phase::Done,
        place: Place::Cloud,
    },
    Agent {
        name: "perf-trace",
        headline: "Reading the flame graph for the slow start",
        phase: Phase::Planning,
        place: Place::Local,
    },
];

const RING_RADIUS: f32 = 11.0;
const RING_CENTRE: f32 = 14.0;
const RING_WIDTH: f32 = 3.0;
const SEGMENT_DEGREES: f32 = 70.0;
const SEGMENT_STEPS: u8 = 12;
const QUARTER_DEGREES: f32 = 90.0;
const SEGMENT_GAP_DEGREES: f32 = 10.0;

const BACKDROP: u32 = 0x000f_1115;
const CARD: u32 = 0x001a_1d24;
const INK: u32 = 0x00e6_e8ee;
const MUTED: u32 = 0x008b_92a5;
const PHASE_INK: u32 = 0x00aa_b1c4;
const ACCENT: u32 = 0x005b_8cff;
const DONE: u32 = 0x004c_c38a;
const UNLIT: u32 = 0x003a_3f4b;
const CHIP: u32 = 0x002a_2e38;

fn ring_segment(origin: gpui::Point<Pixels>, step: u8, ink: Rgba) -> Option<(gpui::Path, Rgba)> {
    let start = QUARTER_DEGREES.mul_add(f32::from(step), -QUARTER_DEGREES) + SEGMENT_GAP_DEGREES;
    let mut builder = PathBuilder::stroke(px(RING_WIDTH));
    builder.style = PathStyle::Stroke(StrokeOptions {
        line_width: RING_WIDTH,
        line_cap: LineCap::Round,
        ..StrokeOptions::default()
    });
    for index in 0..=SEGMENT_STEPS {
        let degrees = SEGMENT_DEGREES.mul_add(f32::from(index) / f32::from(SEGMENT_STEPS), start);
        let radians = degrees.to_radians();
        let at = point(
            origin.x + px(RING_RADIUS.mul_add(radians.cos(), RING_CENTRE)),
            origin.y + px(RING_RADIUS.mul_add(radians.sin(), RING_CENTRE)),
        );
        if index == 0 {
            builder.move_to(at);
        } else {
            builder.line_to(at);
        }
    }
    builder.build().ok().map(|path| (path, ink))
}

fn phase_ring(phase: Phase) -> impl IntoElement {
    let lit = rgb(if phase == Phase::Done { DONE } else { ACCENT });
    let unlit = rgb(UNLIT);
    canvas(
        |_, _, _| (),
        move |bounds: Bounds<Pixels>, (), window: &mut Window, _: &mut App| {
            for step in 0..4 {
                let ink = if step <= phase.step() { lit } else { unlit };
                if let Some((path, ink)) = ring_segment(bounds.origin, step, ink) {
                    window.paint_path(path, Hsla::from(ink));
                }
            }
        },
    )
    .size(px(28.0))
}

fn place_chip(place: Place) -> impl IntoElement {
    let cloud = place == Place::Cloud;
    div()
        .px_1()
        .h(px(16.0))
        .rounded_sm()
        .flex()
        .items_center()
        .text_xs()
        .text_color(rgb(if cloud { INK } else { MUTED }))
        .bg(rgb(if cloud { ACCENT } else { CHIP }))
        .child(if cloud { "cloud" } else { "local" })
}

fn agent_row(agent: &Agent) -> impl IntoElement {
    div()
        .h(px(64.0))
        .px_4()
        .gap_3()
        .rounded_lg()
        .bg(rgb(CARD))
        .flex()
        .items_center()
        .child(phase_ring(agent.phase))
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap_0p5()
                .child(div().text_xs().text_color(rgb(MUTED)).child(agent.name))
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(INK))
                        .child(agent.headline),
                ),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(PHASE_INK))
                .child(agent.phase.label()),
        )
        .child(place_chip(agent.place))
}

struct Fleet;

impl Render for Fleet {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut gpui::Context<'_, Self>,
    ) -> impl IntoElement {
        div()
            .size_full()
            .p_5()
            .gap_3()
            .bg(rgb(BACKDROP))
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(INK))
                            .child("Fleet"),
                    )
                    .child(div().text_xs().text_color(rgb(MUTED)).child("5 agents")),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .children(FLEET.iter().map(agent_row)),
            )
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(640.0), px(440.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Wake fleet (GPUI)".into()),
                    ..TitlebarOptions::default()
                }),
                ..WindowOptions::default()
            },
            |_window, cx| cx.new(|_| Fleet),
        )
        .expect("the window should open");
        cx.activate(true);
    });
}
