# DateField, Calendar y TimeField

Campos de fecha (`dd/mm/aaaa`) y de hora (`HH:MM`, 24 h) con máscara,
validación y un calendario opcional bajo el campo de fecha.

Código: `src/components/date.rs`. La lógica de fechas (`Date`, `Time`,
`month_grid`) vive en `src/datetime.rs` y los textos en español, en
`kai::datetime::names` (un solo lugar, para la i18n pendiente).

> **En desarrollo.** La API puede cambiar mientras se termina el DatePicker;
> esta página describe solo lo que ya existe.

## Uso

```rust
use kai::components::{date_field, time_field};
use kai::datetime::Date;
use kai::forms::{Format, Validators};

date_field("Fecha de inicio", &self.fecha)
    .on_input(Msg::Fecha)
    .today(self.hoy)
    .max(Date::new(2026, 12, 31).unwrap())
    .calendar(self.abierto, self.mes, Msg::Calendario, Msg::Mes, Msg::Elegir)
    .test_id("inicio")
    .build()

time_field("Hora", &self.hora)
    .format(Format::Hora { step: 15 })
    .on_input(Msg::Hora)
    .test_id("hora")
    .build()
```

La app guarda el **texto**: `on_input` recibe `aaaa-mm-dd` (ISO) cuando la
fecha está completa y existe; incompleta o inválida, recibe lo que se ve. El
calendario es estado de la app: si está abierto (`open`) y qué mes muestra
(`shown`). Al elegir un día, la app guarda `fecha.iso()` y lo cierra.

Con un formulario, `FormState::date_field(nombre, etiqueta, Msg::Form)`
arma el `DateField` atado al campo (que se define con `Format::Fecha`), igual
que `FormState::text_field`. `time_field` devuelve un `TextField`, así que
basta con definir el campo con `Format::HORA` y usar `text_field`.

## API

### `date_field(label, value) -> DateField`

| Método | Qué hace |
|---|---|
| `date_field(label, value)` | Campo con `Format::Fecha`, `placeholder("dd/mm/aaaa")` y `Validators::date()`. |
| `on_input(handler)` | Recibe el valor normalizado. |
| `field(\|f\| …)` | Cambia el `TextField` de adentro (ayuda, requerido, error…). |
| `today(fecha)` | Marca el día de hoy en el calendario. Pásalo desde el modelo para que los tests sean deterministas. |
| `min(fecha)`, `max(fecha)` | Deshabilitan días en el calendario. **No validan**: para eso, `Validators::date_range(min, max)`. |
| `first_weekday(Weekday)` | Primer día de la semana (por defecto lunes). |
| `calendar(open, shown, on_toggle, on_month, on_pick)` | Suma el botón «Abrir calendario» y, si `open`, el panel con la grilla. |
| `test_id`, `build()` | — |

### `date_calendar(shown, on_pick, on_month) -> Calendar`

La grilla de un mes sola, para ponerla donde haga falta: encabezado
«marzo de 2026» con mes anterior y siguiente (`on_month` recibe el día 1 del
mes a mostrar) y 6 semanas de días. Métodos: `selected(Option<Date>)`,
`today`, `min`, `max`, `first_weekday`, `test_id`, `build()`.

### `time_field(label, value) -> TextField`

Campo con `Format::HORA` (paso de 1 minuto), `placeholder("HH:MM")` y
`Validators::time()`. Para otro paso, `.format(Format::Hora { step: 15 })`;
para un rango, `.validate(Validators::time_range(min, max))`.

### Formatos y reglas (`kai::forms`)

| | Qué hace |
|---|---|
| `Format::Fecha` | `05/03/2026` en pantalla, `2026-03-05` como valor. Pegar `5/3/2026`, `05-03-2026` o `2026-03-05` sirve. |
| `Format::Hora { step }` / `Format::HORA` | `09:05` en pantalla y como valor. |
| `Validators::date()`, `date_range(min, max)` | Fecha completa y existente; dentro del rango. |
| `Validators::time()`, `time_range(min, max)` | Hora válida; dentro del rango. |

## Teclado

- En el campo, **↑/↓** cambian el día, el mes o el año (fecha) o la hora o
  los minutos según `step` (hora), según dónde esté el cursor.
- Con el calendario abierto, el panel es modal: toma el foco y lo devuelve
  al campo al cerrar. **Escape** lo cierra.
- Los días son un grupo de foco: **Tab** entra en el día elegido (o en el
  primero), **←/→** pasan al día anterior o siguiente y **Return** elige.
  Hoy **↑/↓** hacen lo mismo que ←/→ (un día).

**Pendiente:** ↑/↓ por semana (7 días) dentro de la grilla, cruzar de mes
con las flechas, RePág/AvPág para cambiar de mes e Inicio/Fin de semana.
Hoy las flechas recorren los días en orden y se detienen en los extremos
de la grilla.

## Accesibilidad

El panel es un `Role::Dialog` llamado «Calendario»; la grilla, un grupo con
el mismo nombre. Cada día es un botón con su nombre largo («jueves 5 de
marzo de 2026», más «, hoy» si corresponde), marcado como presionado si es
el elegido y deshabilitado fuera de `min`/`max`.

## Partes

`DateField`: las del `TextField` (`x`, `x.input`, `x.error`…) más `x.open`
(botón) y `x.calendar` (panel), con la grilla en `x.calendar.grid`.

`Calendar`: `x.prev`, `x.next`, `x.title` y `x.day.<aaaa-mm-dd>`.

## Limitaciones

- El panel se dibuja con `absolute z-50` bajo el campo: un contenedor con
  `overflow-hidden` o con scroll lo recorta. Falta una capa flotante común
  (la misma que necesitará el autocompletado).
- Sin zona horaria: `Date::today_utc()` es el día en UTC.

## Relacionados

[TextField](text-field.md) (formatos, `validate`, `validate_on`) ·
[Elementos comunes](elementos-comunes.md)
