//! The Wake fleet view on Rune: five fixed agents, each with a headline, a phase and where it runs.

use rune::color::hex;
use rune::tok::type_;
use rune::{App, AppSpec, Color, Cx, El, LineCap, Shape, Titlebar, canvas, col, el, row, text};
use rune_kit::{Badge, BadgeVariant, Kit};

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

const TITLEBAR_HEIGHT: f32 = 36.0;
const RING_RADIUS: f32 = 11.0;
const RING_CENTRE: f32 = 14.0;
const RING_WIDTH: f32 = 3.0;
const SEGMENT_DEGREES: f32 = 70.0;
const SEGMENT_STEPS: u8 = 12;

fn colour(value: &str) -> Color {
    hex(value).unwrap_or_default()
}

fn arc(start_degrees: f32) -> Vec<[f32; 2]> {
    (0..=SEGMENT_STEPS)
        .map(|index| {
            let degrees =
                SEGMENT_DEGREES.mul_add(f32::from(index) / f32::from(SEGMENT_STEPS), start_degrees);
            let radians = degrees.to_radians();
            [
                RING_CENTRE + RING_RADIUS * radians.cos(),
                RING_CENTRE + RING_RADIUS * radians.sin(),
            ]
        })
        .collect()
}

fn phase_ring(phase: Phase) -> El {
    let lit = if phase == Phase::Done {
        colour("#4cc38a")
    } else {
        colour("#5b8cff")
    };
    let unlit = colour("#3a3f4b");
    let shape = (0..4).fold(Shape::new(), |shape, step| {
        let start = 90.0_f32.mul_add(f32::from(step), -90.0) + 10.0;
        let ink = if step <= phase.step() { lit } else { unlit };
        shape.stroke_with(arc(start), false, RING_WIDTH, LineCap::Round, ink)
    });
    canvas(shape).size(28.0, 28.0)
}

fn agent_row(agent: &Agent, kit: &Kit) -> El {
    let (variant, place) = match agent.place {
        Place::Cloud => (BadgeVariant::Accent, "cloud"),
        Place::Local => (BadgeVariant::Neutral, "local"),
    };
    let words = col()
        .grow(1.0)
        .gap(2.0)
        .child(text(agent.name).font(type_::LABEL).color(colour("#8b92a5")))
        .child(
            text(agent.headline)
                .font(type_::TITLE)
                .color(colour("#e6e8ee")),
        );
    row()
        .h(64.0)
        .px(16.0)
        .gap(14.0)
        .center_y()
        .radius(10.0)
        .fill(colour("#1a1d24"))
        .child(phase_ring(agent.phase))
        .child(words)
        .child(
            text(agent.phase.label())
                .font(type_::LABEL)
                .color(colour("#aab1c4")),
        )
        .child(Badge::new(place).variant(variant).build(kit))
}

struct Fleet {
    agents: &'static [Agent],
}

fn view(fleet: &Fleet, _: &mut Cx<Fleet>) -> El {
    let kit = Kit::neutral(true);
    let mut list = col().gap(8.0);
    for agent in fleet.agents {
        list = list.child(agent_row(agent, &kit));
    }
    col()
        .w_full()
        .h_full()
        .px(20.0)
        .pb(20.0)
        .gap(14.0)
        .fill(colour("#0f1115"))
        .child(el().h(TITLEBAR_HEIGHT).window_drag())
        .child(
            row()
                .center_y()
                .child(
                    text("Fleet")
                        .font(type_::TITLE)
                        .color(colour("#e6e8ee"))
                        .grow(1.0),
                )
                .child(text("5 agents").font(type_::LABEL).color(colour("#8b92a5"))),
        )
        .child(list)
        .child(el().grow(1.0))
}

fn main() {
    let spec = AppSpec::new(
        "Wake fleet (Rune)",
        (640.0, 470.0),
        || Fleet { agents: &FLEET },
        view,
    );
    App::new(spec)
        .titlebar(Titlebar::new(TITLEBAR_HEIGHT))
        .run();
}
