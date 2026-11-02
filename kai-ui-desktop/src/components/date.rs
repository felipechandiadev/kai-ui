//! Fechas y horas: [`date_field`] (campo `dd/mm/aaaa` con calendario),
//! [`date_calendar`] (la grilla del mes) y [`time_field`] (`HH:MM`).
//! La lógica de calendario vive en [`crate::datetime`]; los nombres en
//! español, en [`crate::datetime::names`].

use std::rc::Rc;

use super::{IconButtonSize, IconButtonVariant, TextField, icon_button_sized, themed};
use crate::datetime::{Date, Weekday, month_grid, names};
use crate::element::{Element, Role, div, text};
use crate::forms::{Format, FormMsg, FormState, Validators};
use crate::icons::Icon;
use crate::overlay::Floating;

/// Grilla de un mes para elegir un día ([`date_calendar`]).
pub struct Calendar<Msg> {
    shown: Date,
    selected: Option<Date>,
    today: Option<Date>,
    min: Option<Date>,
    max: Option<Date>,
    first_weekday: Weekday,
    on_pick: Rc<dyn Fn(Date) -> Msg>,
    on_month: Rc<dyn Fn(Date) -> Msg>,
    test_id: Option<String>,
}

/// Calendario del mes de `shown`: encabezado «marzo de 2026» con mes
/// anterior y siguiente (`on_month` recibe el día 1 del mes a mostrar) y
/// 6 semanas de días; un clic, o Return sobre el día con foco, envía
/// `on_pick(día)`. Los días son un grupo de foco: Tab entra en el elegido
/// (o en el primero) y las flechas recorren los días.
pub fn date_calendar<Msg>(
    shown: Date,
    on_pick: impl Fn(Date) -> Msg + 'static,
    on_month: impl Fn(Date) -> Msg + 'static,
) -> Calendar<Msg> {
    Calendar {
        shown: shown.first_of_month(),
        selected: None,
        today: None,
        min: None,
        max: None,
        first_weekday: Weekday::Monday,
        on_pick: Rc::new(on_pick),
        on_month: Rc::new(on_month),
        test_id: None,
    }
}

impl<Msg> Calendar<Msg> {
    pub fn selected(mut self, date: Option<Date>) -> Self {
        self.selected = date;
        self
    }

    /// El día que se marca como hoy. Pásalo desde el modelo (o
    /// [`Date::today_utc`]) para que los tests sean deterministas.
    pub fn today(mut self, today: Date) -> Self {
        self.today = Some(today);
        self
    }

    /// Días anteriores quedan deshabilitados.
    pub fn min(mut self, min: Date) -> Self {
        self.min = Some(min);
        self
    }

    /// Días posteriores quedan deshabilitados.
    pub fn max(mut self, max: Date) -> Self {
        self.max = Some(max);
        self
    }

    /// Primer día de la semana (por defecto lunes, es-CL).
    pub fn first_weekday(mut self, weekday: Weekday) -> Self {
        self.first_weekday = weekday;
        self
    }

    /// Partes: `<id>.prev`, `<id>.next`, `<id>.title` y `<id>.day.<aaaa-mm-dd>`.
    pub fn test_id(mut self, id: impl Into<String>) -> Self {
        self.test_id = Some(id.into());
        self
    }

    fn allowed(&self, date: Date) -> bool {
        self.min.is_none_or(|min| date >= min) && self.max.is_none_or(|max| date <= max)
    }

    pub fn build(self) -> Element<Msg> {
        let id = |part: &str| self.test_id.as_ref().map(|id| format!("{id}.{part}"));
        let with_id = |element: Element<Msg>, part: &str| match id(part) {
            Some(test_id) => element.test_id(test_id),
            None => element,
        };
        let shown = self.shown;
        let month = |delta: i32| shown.add_months(delta).unwrap_or(shown);
        let header = div()
            .class("flex items-center justify-between gap-2")
            .children([
                with_id(
                    icon_button_sized(
                        Icon::ChevronLeft,
                        names::PREVIOUS_MONTH,
                        IconButtonVariant::Neutral,
                        IconButtonSize::Sm,
                    )
                    .on_click((self.on_month)(month(-1))),
                    "prev",
                ),
                with_id(
                    text(format!(
                        "{} de {}",
                        names::MONTHS[usize::from(shown.month()) - 1],
                        shown.year()
                    ))
                    .class(&themed(
                        "text-sm font-semibold text-foreground",
                        "text-control font-semibold text-label",
                    ))
                    .role(Role::Heading),
                    "title",
                ),
                with_id(
                    icon_button_sized(
                        Icon::ChevronRight,
                        names::NEXT_MONTH,
                        IconButtonVariant::Neutral,
                        IconButtonSize::Sm,
                    )
                    .on_click((self.on_month)(month(1))),
                    "next",
                ),
            ]);
        let weekdays = div().class("grid grid-cols-7 gap-0.5").children((0..7).map(|i| {
            let day = Weekday::from_index(self.first_weekday.index() + i);
            text(names::WEEKDAY_INITIALS[day.index()])
                .class(&themed(
                    "text-center text-xs text-muted-foreground",
                    "text-center text-xs text-secondary-label",
                ))
                .label(day.name())
        }));
        let day_class = themed(
            "flex items-center justify-center h-8 rounded-md text-sm text-foreground \
             cursor-pointer hover:bg-neutral selected:bg-choice/20 selected:border \
             selected:border-choice focus-visible:ring-2 focus-visible:ring-primary/40 \
             disabled:opacity-40 disabled:cursor-not-allowed",
            "flex items-center justify-center h-7 rounded-control text-control text-label \
             cursor-default hover:bg-fill-3 selected:bg-choice/20 selected:border \
             selected:border-choice keyboard-focus:ring-focus disabled:text-disabled-control-text",
        );
        let days = div()
            .class("grid grid-cols-7 gap-0.5")
            .role(Role::Group)
            .label(names::CALENDAR)
            .focus_group()
            .children(month_grid(shown, self.first_weekday).into_iter().map(|date| {
                let outside = date.first_of_month() != shown;
                let is_today = self.today == Some(date);
                let mut classes = day_class.clone();
                if outside {
                    classes.push_str(" opacity-50");
                }
                if is_today {
                    classes.push_str(" font-bold text-primary");
                }
                let mut name = date.long_name();
                if is_today {
                    name.push_str(&format!(", {}", names::TODAY));
                }
                let day = div()
                    .class(&classes)
                    .role(Role::Button)
                    .key(date.iso())
                    .label(name)
                    // El grupo entra por el día elegido (`On`).
                    .pressed(self.selected == Some(date))
                    .disabled(!self.allowed(date))
                    .on_click((self.on_pick)(date))
                    .child(text(date.day().to_string()));
                with_id(day, &format!("day.{}", date.iso()))
            }));
        let root = div()
            .class("flex flex-col gap-2")
            .children([header, weekdays, days]);
        match &self.test_id {
            Some(id) => root.test_id(id.clone()),
            None => root,
        }
    }
}

/// Estado del calendario de un [`DateField`], que guarda la app.
struct Popover<Msg> {
    open: bool,
    shown: Date,
    on_toggle: Msg,
    on_month: Rc<dyn Fn(Date) -> Msg>,
    on_pick: Rc<dyn Fn(Date) -> Msg>,
}

/// Campo de fecha `dd/mm/aaaa` ([`date_field`]).
pub struct DateField<Msg> {
    field: TextField<Msg>,
    selected: Option<Date>,
    today: Option<Date>,
    min: Option<Date>,
    max: Option<Date>,
    first_weekday: Weekday,
    popover: Option<Popover<Msg>>,
    test_id: Option<String>,
}

/// Campo de fecha: máscara `dd/mm/aaaa` ([`Format::Fecha`]), `on_input`
/// recibe `aaaa-mm-dd` cuando la fecha está completa y es válida, regla
/// [`Validators::date`] y ↑/↓ para cambiar día, mes o año. Con
/// [`DateField::calendar`] suma un botón con icono de calendario que abre
/// la grilla del mes en un panel bajo el campo.
pub fn date_field<Msg>(label: impl Into<String>, value: impl Into<String>) -> DateField<Msg> {
    let value = value.into();
    DateField {
        selected: Date::parse(&value),
        field: TextField::new(label, value)
            .format(Format::Fecha)
            .placeholder("dd/mm/aaaa")
            .validate(Validators::date()),
        today: None,
        min: None,
        max: None,
        first_weekday: Weekday::Monday,
        popover: None,
        test_id: None,
    }
}

impl<Msg: Clone + 'static> DateField<Msg> {
    /// Cambia el [`TextField`] de adentro (ayuda, requerido, error…).
    pub fn field(mut self, edit: impl FnOnce(TextField<Msg>) -> TextField<Msg>) -> Self {
        self.field = edit(self.field);
        self
    }

    pub fn on_input(self, handler: impl Fn(String) -> Msg + 'static) -> Self {
        self.field(|f| f.on_input(handler))
    }

    /// Partes: las del [`TextField`] más `<id>.open` (botón) y
    /// `<id>.calendar` (panel), con las del [`Calendar`] adentro.
    pub fn test_id(mut self, id: impl Into<String>) -> Self {
        let id = id.into();
        self.field = self.field.test_id(id.clone());
        self.test_id = Some(id);
        self
    }

    pub fn today(mut self, today: Date) -> Self {
        self.today = Some(today);
        self
    }

    /// Mínimo: deshabilita días en el calendario. Para validar, suma
    /// `Validators::date_range`.
    pub fn min(mut self, min: Date) -> Self {
        self.min = Some(min);
        self
    }

    pub fn max(mut self, max: Date) -> Self {
        self.max = Some(max);
        self
    }

    pub fn first_weekday(mut self, weekday: Weekday) -> Self {
        self.first_weekday = weekday;
        self
    }

    /// Botón de calendario y panel. La app guarda si está abierto y el mes
    /// que muestra: `on_toggle` abre o cierra (también Escape), `on_month`
    /// cambia de mes y `on_pick` elige un día (la app guarda `fecha.iso()`
    /// y cierra). Mientras está abierto, el panel flota bajo el campo
    /// (docs/dev/32): toma el foco y lo devuelve al cerrar, y un clic
    /// afuera también lo cierra.
    pub fn calendar(
        mut self,
        open: bool,
        shown: Date,
        on_toggle: Msg,
        on_month: impl Fn(Date) -> Msg + 'static,
        on_pick: impl Fn(Date) -> Msg + 'static,
    ) -> Self {
        self.popover = Some(Popover {
            open,
            shown,
            on_toggle,
            on_month: Rc::new(on_month),
            on_pick: Rc::new(on_pick),
        });
        self
    }

    pub fn build(self) -> Element<Msg> {
        let Some(popover) = self.popover else {
            return self.field.build();
        };
        let id = |part: &str| self.test_id.as_ref().map(|id| format!("{id}.{part}"));
        let mut open_button = icon_button_sized(
            Icon::CalendarDays,
            names::OPEN_CALENDAR,
            IconButtonVariant::Neutral,
            IconButtonSize::Sm,
        )
        .pressed(popover.open)
        .on_click(popover.on_toggle.clone());
        if let Some(test_id) = id("open") {
            open_button = open_button.test_id(test_id);
        }
        let field = self.field.end_adornment(open_button).build();
        if !popover.open {
            return field;
        }
        let on_month = Rc::clone(&popover.on_month);
        let on_pick = Rc::clone(&popover.on_pick);
        let mut calendar = date_calendar(popover.shown, move |d| on_pick(d), move |d| on_month(d))
            .selected(self.selected)
            .first_weekday(self.first_weekday);
        if let Some(today) = self.today {
            calendar = calendar.today(today);
        }
        if let Some(min) = self.min {
            calendar = calendar.min(min);
        }
        if let Some(max) = self.max {
            calendar = calendar.max(max);
        }
        if let Some(test_id) = id("calendar") {
            calendar = calendar.test_id(format!("{test_id}.grid"));
        }
        let mut panel = div()
            .class(&themed(
                "w-[288px] p-3 rounded-md border border-border bg-background shadow-lg",
                "w-[272px] p-2 rounded-control border border-separator bg-control shadow-lg",
            ))
            .role(Role::Dialog)
            .label(names::CALENDAR)
            .child(calendar.build());
        if let Some(test_id) = id("calendar") {
            panel = panel.test_id(test_id);
        }
        // Capa flotante (docs/dev/32): sobre el resto de la ventana, sin
        // recorte de los ancestros, bajo el campo o sobre él si no cabe.
        field.child(
            panel.floating(
                Floating::below()
                    .contain_focus()
                    .on_dismiss(popover.on_toggle),
            ),
        )
    }
}

/// Campo de hora `HH:MM` (24 h, [`Format::HORA`]): regla
/// [`Validators::time`] y ↑/↓ para cambiar la hora o el minuto según el
/// cursor. Para otro paso: `.format(Format::Hora { step: 15 })`; para un
/// rango, `.validate(Validators::time_range(min, max))`.
pub fn time_field<Msg>(label: impl Into<String>, value: impl Into<String>) -> TextField<Msg> {
    TextField::new(label, value)
        .format(Format::HORA)
        .placeholder("HH:MM")
        .validate(Validators::time())
}

/// Campos de fecha y hora atados a un [`FormState`].
impl FormState {
    /// [`date_field`] del campo `name` ([`FormState::text_field`] con la
    /// máscara de fecha). Define el campo con `Format::Fecha`.
    pub fn date_field<Msg: Clone + 'static>(
        &self,
        name: &str,
        label: impl Into<String>,
        wrap: fn(FormMsg) -> Msg,
    ) -> DateField<Msg> {
        let field = self.text_field(name, label, wrap).placeholder("dd/mm/aaaa");
        DateField {
            selected: self.value(name).and_then(Date::parse),
            field,
            today: None,
            min: None,
            max: None,
            first_weekday: Weekday::Monday,
            popover: None,
            test_id: Some(name.to_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{date_field, time_field};
    use crate::datetime::Date;
    use crate::element::{Element, Role, div};
    use crate::forms::{Format, Validators};
    use crate::runtime::SimpleApp;
    use crate::testing::{By, MemoryClipboard, TestApp};
    use crate::ui_engine::{Key, Modifiers};

    fn d(y: i32, m: u8, day: u8) -> Date {
        Date::new(y, m, day).unwrap()
    }

    struct Agenda {
        date: String,
        time: String,
        open: bool,
        shown: Date,
    }

    impl Default for Agenda {
        fn default() -> Self {
            Self {
                date: String::new(),
                time: String::new(),
                open: false,
                shown: d(2026, 3, 1),
            }
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Date(String),
        Time(String),
        Toggle,
        Month(Date),
        Pick(Date),
    }

    impl SimpleApp for Agenda {
        type Msg = Msg;
        fn title(&self) -> String {
            "Agenda".into()
        }
        fn update(&mut self, msg: Msg) {
            match msg {
                Msg::Date(value) => self.date = value,
                Msg::Time(value) => self.time = value,
                Msg::Toggle => self.open = !self.open,
                Msg::Month(month) => self.shown = month,
                Msg::Pick(date) => {
                    self.date = date.iso();
                    self.open = false;
                }
            }
        }
        fn view(&self) -> Element<Msg> {
            div().class("flex flex-col gap-2 p-4").children([
                date_field("Fecha", &self.date)
                    .on_input(Msg::Date)
                    .today(d(2026, 3, 10))
                    .max(d(2026, 4, 15))
                    .calendar(self.open, self.shown, Msg::Toggle, Msg::Month, Msg::Pick)
                    .test_id("fecha")
                    .build(),
                time_field("Hora", &self.time)
                    .format(Format::Hora { step: 15 })
                    .validate(Validators::time())
                    .on_input(Msg::Time)
                    .test_id("hora")
                    .build(),
            ])
        }
    }

    fn cmd_v(app: &mut TestApp<Agenda>, clipboard: &MemoryClipboard, text: &str) {
        clipboard.set(text);
        app.press(
            Key::V,
            Modifiers {
                command: true,
                ..Modifiers::NONE
            },
        );
    }

    #[test]
    fn typing_and_pasting_a_date_reports_iso() {
        let clipboard = MemoryClipboard::new();
        let mut app = TestApp::builder(Agenda::default())
            .clipboard(clipboard.clone())
            .build();
        app.type_text(By::test_id("fecha"), "0503202");
        assert_eq!(app.value(By::test_id("fecha")), "05/03/202");
        assert_eq!(app.app().date, "05/03/202", "incompleta: lo que se ve");
        app.type_text(By::test_id("fecha"), "6");
        assert_eq!(app.value(By::test_id("fecha")), "05/03/2026");
        assert_eq!(app.app().date, "2026-03-05");

        app.press(
            Key::A,
            Modifiers {
                command: true,
                ..Modifiers::NONE
            },
        );
        cmd_v(&mut app, &clipboard, "2026-12-24");
        assert_eq!(app.value(By::test_id("fecha")), "24/12/2026");
        assert_eq!(app.app().date, "2026-12-24");
    }

    #[test]
    fn arrows_step_the_day_month_or_year_under_the_caret() {
        let mut app = TestApp::new(Agenda::default());
        app.type_text(By::test_id("fecha"), "31012026");
        app.press(Key::ArrowUp, Modifiers::NONE);
        assert_eq!(app.value(By::test_id("fecha")), "31/01/2027", "cursor en el año");
        app.press(Key::Home, Modifiers::NONE);
        app.press(Key::ArrowUp, Modifiers::NONE);
        assert_eq!(app.value(By::test_id("fecha")), "01/02/2027");
        assert_eq!(app.app().date, "2027-02-01");
        app.press(Key::ArrowDown, Modifiers::NONE);
        assert_eq!(app.app().date, "2027-01-31");
    }

    #[test]
    fn an_impossible_date_fails_validation_after_blur() {
        let mut app = TestApp::new(Agenda::default());
        app.type_text(By::test_id("fecha"), "30022026");
        assert!(app.screen().query(By::text("Fecha no válida")).is_none());
        app.click(By::test_id("hora"));
        app.screen().get(By::text("Fecha no válida"));
    }

    #[test]
    fn the_time_field_masks_and_steps_by_its_step() {
        let mut app = TestApp::new(Agenda::default());
        app.type_text(By::test_id("hora"), "0905");
        assert_eq!(app.value(By::test_id("hora")), "09:05");
        app.press(Key::ArrowUp, Modifiers::NONE);
        assert_eq!(app.app().time, "09:15");
        app.press(Key::Home, Modifiers::NONE);
        app.press(Key::ArrowDown, Modifiers::NONE);
        assert_eq!(app.app().time, "08:15");
        app.press(Key::End, Modifiers::NONE);
        app.press(Key::Backspace, Modifiers::NONE);
        app.press(Key::Backspace, Modifiers::NONE);
        app.type_text(By::test_id("hora"), "99");
        app.click(By::test_id("fecha"));
        app.screen().get(By::text("Hora no válida"));
    }

    #[test]
    fn the_calendar_opens_navigates_and_picks_a_day() {
        let mut app = TestApp::new(Agenda::default());
        assert!(app.screen().query(By::test_id("fecha.calendar")).is_none());
        app.click(By::test_id("fecha.open"));
        let panel = app.screen().get(By::role(Role::Dialog).name("Calendario"));
        assert_eq!(panel.test_id(), Some("fecha.calendar"));
        app.screen().get(By::text("marzo de 2026"));
        app.screen()
            .get(By::role(Role::Button).name("martes 10 de marzo de 2026, hoy"));
        // La semana parte el lunes: marzo 2026 empieza en domingo, la
        // grilla parte el lunes 23 de febrero.
        app.screen().get(By::test_id("fecha.calendar.grid.day.2026-02-23"));

        app.click(By::test_id("fecha.calendar.grid.next"));
        app.screen().get(By::text("abril de 2026"));
        assert!(!app.is_enabled(By::test_id("fecha.calendar.grid.day.2026-04-16")), "pasado el máximo");
        app.click(By::test_id("fecha.calendar.grid.day.2026-04-07"));
        assert_eq!(app.app().date, "2026-04-07");
        assert_eq!(app.value(By::test_id("fecha")), "07/04/2026");
        assert!(app.screen().query(By::test_id("fecha.calendar")).is_none());
    }

    #[test]
    fn escape_closes_the_calendar_and_focus_returns() {
        let mut app = TestApp::new(Agenda {
            date: "2026-03-05".into(),
            ..Agenda::default()
        });
        app.click(By::test_id("fecha.open"));
        assert!(!app.is_focused(By::test_id("fecha.open")), "el panel toma el foco");
        // Tab entra en la grilla por el día elegido; las flechas recorren.
        let day = |iso: &str| By::test_id(format!("fecha.calendar.grid.day.{iso}"));
        for _ in 0..4 {
            if app.is_focused(day("2026-03-05")) {
                break;
            }
            app.press(Key::Tab, Modifiers::NONE);
        }
        assert!(app.is_focused(day("2026-03-05")));
        app.press(Key::ArrowRight, Modifiers::NONE);
        assert!(app.is_focused(day("2026-03-06")));
        app.press(Key::Enter, Modifiers::NONE);
        assert_eq!(app.app().date, "2026-03-06");

        app.click(By::test_id("fecha.open"));
        app.press(Key::Escape, Modifiers::NONE);
        assert!(!app.app().open);
        assert!(app.is_focused(By::test_id("fecha.open")), "el foco vuelve");
    }
}
