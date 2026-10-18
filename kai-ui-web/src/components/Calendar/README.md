# Calendar

Calendario de mes y semana, más helpers de fechas (`formatIsoDate`, `getWeekStart`, `addDaysIso`, etc.).

## Import

```ts
import { Calendar, type CalendarEvent } from "@felipechandiadev/ui";
```

## Props

- `view`: `month` o `week`. Obligatorio.
- `events`: lista de `CalendarEvent` (`id`, `date` ISO, `content`).
- `weekStartsOn`: `0` domingo o `1` lunes (default).
- `columnsFrom`: desde qué breakpoint la semana muestra columnas. Default `lg`.

## Servidor

Client component. Next lo incluye en el HTML y lo hidrata.

## Ejemplo mínimo

```tsx
<Calendar view="month" events={[]} />
```
