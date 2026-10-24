# Switch

Interruptor de sí/no (`.fs-switch`).

Código: `src/components.rs` (`switch`, `switch_labeled`, `switch_with_options`).

## Uso

```rust
use kai::components::{switch, switch_labeled, SwitchDensity};

switch(self.activo, Msg::AlternarActivo)

switch_labeled("Notificaciones", self.avisos, Msg::AlternarAvisos, false, SwitchDensity::Default)
```

## API

| Firma | Qué hace |
|---|---|
| `switch(checked, on_toggle)` | Interruptor estándar. |
| `switch_labeled(label, checked, on_toggle, disabled, density)` | Con etiqueta. |
| `switch_with_options(checked, on_toggle, disabled, density)` | Sin etiqueta, configurable. |

`SwitchDensity`: `Default`, `Compact`. El mensaje es el mismo al encender y
al apagar: la app invierte su valor.

## Accesibilidad

Rol `Switch` con estado encendido/apagado. Espacio lo alterna. Sin
`switch_labeled`, nómbralo con `.label(..)`.

## Switch o Checkbox

Switch para un ajuste que se aplica al momento; [Checkbox](checkbox.md) para
una opción de un formulario que se envía después.
