//! An app that only uses Kai's public API.

use kai::components::{ButtonVariant, TextField, button, card, card_content, heading, paragraph};
use kai::element::{Element, div};
use kai::runtime::{self, App, Effects};
use kai::theme::{Density, Theme};

#[derive(Default)]
struct Greeter {
    name: String,
    greeting: Option<String>,
}

#[derive(Clone, Debug)]
enum Msg {
    NameChanged(String),
    Greet,
}

impl App for Greeter {
    type Msg = Msg;

    fn title(&self) -> String {
        "Kai — app externa".to_owned()
    }

    fn init(&mut self) -> Effects<Msg> {
        self.greeting = Some("Escribe tu nombre y pulsa Saludar".to_owned());
        Effects::none()
    }

    fn update(&mut self, msg: Msg) -> Effects<Msg> {
        match msg {
            Msg::NameChanged(name) => self.name = name,
            Msg::Greet => self.greeting = Some(format!("Hola, {}", self.name.trim())),
        }
        Effects::none()
    }

    fn view(&self) -> Element<Msg> {
        div()
            .class("w-full h-full flex items-center justify-center bg-neutral appkit:bg-window")
            .child(
                card().class("w-96").child(
                    card_content().class("flex flex-col gap-4").children([
                        heading("App externa"),
                        TextField::new("Nombre", self.name.clone())
                            .on_input(Msg::NameChanged)
                            .on_submit(Msg::Greet)
                            .build(),
                        button("Saludar", ButtonVariant::Primary).on_click(Msg::Greet),
                        paragraph(self.greeting.clone().unwrap_or_default()),
                    ]),
                ),
            )
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    runtime::run(Greeter::default(), Theme::kai(Density::Comfortable))
}
