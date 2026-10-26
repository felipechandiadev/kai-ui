//! /selection: switches, checkboxes with a mixed state, selects and
//! segmented controls.

use kai::components::{
    SwitchDensity, card, card_content, card_header, checkbox_with_options, page, page_body,
    segmented_with_options, select_with_options, switch, switch_with_options,
};
use kai::element::{ControlState, Element, div, text};
use kai::router::{Context, Page};
use kai::runtime::Effects;

use crate::state::State;

const COUNTRIES: [&str; 4] = ["Chile", "Argentina", "Perú", "Uruguay"];
const ALIGNMENTS: [&str; 3] = ["Izquierda", "Centro", "Derecha"];
/// Permissions under «Todos».
const PERMISSIONS: [&str; 3] = ["Leer", "Escribir", "Compartir"];

pub struct Screen {
    wifi: bool,
    permissions: [bool; 3],
    country: Option<usize>,
    alignment: usize,
}

#[derive(Clone, Debug)]
pub enum Msg {
    ToggleWifi,
    /// One permission, or all of them (`None`, «Todos»).
    TogglePermission(Option<usize>),
    Country(usize),
    Alignment(usize),
}

impl Page for Screen {
    type Msg = Msg;
    type Params = ();
    type State = State;

    fn init(_: &(), _: &State) -> (Self, Effects<Msg>) {
        let screen = Self {
            wifi: true,
            permissions: [true, false, true],
            country: Some(0),
            alignment: 0,
        };
        (screen, Effects::none())
    }

    fn update(&mut self, msg: Msg, _: &mut Context<'_, State>) -> Effects<Msg> {
        match msg {
            Msg::ToggleWifi => self.wifi = !self.wifi,
            Msg::TogglePermission(Some(index)) => {
                self.permissions[index] = !self.permissions[index];
            }
            Msg::TogglePermission(None) => {
                let all = !self.permissions.iter().all(|on| *on);
                self.permissions = [all; 3];
            }
            Msg::Country(index) => self.country = Some(index),
            Msg::Alignment(index) => self.alignment = index,
        }
        Effects::none()
    }

    fn view(&self, _: &State) -> Element<Msg> {
        page().child(page_body().child(self.selection()))
    }

    fn title(&self, _: &State) -> String {
        "Selección".into()
    }
}

impl Screen {
    fn selection(&self) -> Element<Msg> {
        let state = |on: bool| {
            if on {
                ControlState::On
            } else {
                ControlState::Off
            }
        };
        let all = match self.permissions.iter().filter(|on| **on).count() {
            0 => ControlState::Off,
            3 => ControlState::On,
            _ => ControlState::Mixed,
        };
        let label = |content: &str| {
            text(content).class("text-sm font-medium text-foreground appkit:text-label")
        };
        let mut checks = vec![
            checkbox_with_options("Todos", all, Msg::TogglePermission(None), false)
                .test_id("selection.all"),
        ];
        for (index, name) in PERMISSIONS.iter().enumerate() {
            checks.push(
                checkbox_with_options(
                    name,
                    state(self.permissions[index]),
                    Msg::TogglePermission(Some(index)),
                    false,
                )
                .class("ml-6")
                .test_id(format!("selection.permission.{index}")),
            );
        }
        checks.push(
            checkbox_with_options(
                "Administrar",
                ControlState::Off,
                Msg::TogglePermission(None),
                true,
            )
            .class("ml-6")
            .test_id("selection.admin"),
        );
        card()
            .class("w-full max-w-3xl")
            .test_id("selection")
            .children([
                card_header(
                    "Selección",
                    Some("Switch, checkbox con estado mixto, select y control segmentado"),
                ),
                card_content().class("gap-5").children([
                    div().class("flex flex-wrap items-center gap-6").children([
                        div().class("flex items-center gap-2").children([
                            switch(self.wifi, Msg::ToggleWifi)
                                .label("Wi-Fi")
                                .test_id("selection.wifi"),
                            label("Wi-Fi"),
                        ]),
                        div().class("flex items-center gap-2").children([
                            switch_with_options(
                                true,
                                Msg::ToggleWifi,
                                true,
                                SwitchDensity::Compact,
                            )
                            .label("Bluetooth")
                            .test_id("selection.bluetooth"),
                            label("Bluetooth (deshabilitado)"),
                        ]),
                    ]),
                    div()
                        .class("flex flex-col items-start gap-2")
                        .children(checks),
                    div()
                        .class("grid grid-cols-1 sm:grid-cols-2 gap-4")
                        .children([
                            select_with_options(
                                &COUNTRIES,
                                self.country,
                                "Elegir país",
                                Msg::Country,
                                false,
                            )
                            .label("País")
                            .test_id("selection.country"),
                            select_with_options(
                                &COUNTRIES,
                                None,
                                "Sin región",
                                Msg::Country,
                                true,
                            )
                            .label("Región")
                            .test_id("selection.region"),
                        ]),
                    div().class("flex flex-wrap items-center gap-3").children([
                        label("Alineación"),
                        segmented_with_options(&ALIGNMENTS, self.alignment, Msg::Alignment, false)
                            .label("Alineación")
                            .test_id("selection.alignment"),
                        segmented_with_options(
                            &["Día", "Semana"],
                            0,
                            |_| Msg::ToggleWifi,
                            true,
                        )
                        .label("Vista")
                        .test_id("selection.view"),
                    ]),
                ]),
            ])
    }
}
