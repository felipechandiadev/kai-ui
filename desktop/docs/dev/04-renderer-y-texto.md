# 04 — Renderer y texto

**Fase:** L5 (capa de renderizado desacoplada). El trabajo de renderer desacoplado comienza después de que L0–L4 ofrezcan contratos estables.

## Objetivo

Convertir el árbol ya medido en una lista de operaciones visuales que Kai presenta en una superficie GPU, manteniendo el dibujo de todos los controles dentro del renderer propio.

## División de responsabilidades

- **Plataforma/ventana:** superficie, escala, eventos y ciclo de vida.
- **Layout:** calcula cajas, clipping y restricciones.
- **Árbol de widgets:** produce nodos visuales/operaciones semánticas de dibujo.
- **Renderer:** transforma esas operaciones en recursos y comandos GPU; no decide reglas de negocio ni propiedad del estado.
- **Texto:** shaping, fuentes, edición, caret, selección, composición y métricas, coordinados con el renderer.

## Pasos

1. Definir operaciones de dibujo independientes de `wgpu`: rectángulo, borde, clip, texto y orden de capas.
2. Encapsular surface/device/pipeline/buffers y recursos GPU en el backend; validar Metal, resize, formato, recreación y errores.
3. Hacer que L6 produzca una lista de dibujo; el backend la interpreta sin asumir estado de negocio ni API pública GPU.
4. Implementar clipping y orden de capas antes de sombras, filtros y efectos avanzados.
5. Mantener `glyphon` como presentación de texto y `cosmic-text::Editor` para edición hasta que haya evidencia para cambiar. La entrada `winit::Ime::Preedit`/`Commit` conserva su frontera en plataforma y debe pasar pruebas macOS explícitas.
	- Los buffers y layout de Kai se expresan en unidades lógicas. Al crear `TextArea`, convertir los orígenes y límites al espacio físico de la superficie; aplicar el factor de escala una sola vez a la escala del texto y a su posición.
	- La edición activa IME mientras el campo tiene foco y lo desactiva al perderlo. El clipboard está adaptado por `arboard`; probar fallos/permisos del sistema y no bloquear el ciclo gráfico con trabajo lento.
6. Definir política de recursos: carga de fuentes, texturas, cacheo de glifos, invalidación y destrucción al cerrar/cambiar de escala.
7. Evitar trabajo GPU y asignaciones innecesarias por frame solo tras medir; registrar benchmarks en hardware y versión conocidos.
8. Añadir capturas/pruebas visuales para primitivas y widgets base; tolerar diferencias de rasterización con comparaciones adecuadas.

## Estado actual de L5

### Operaciones de dibujo (`render/mod.rs`)

- `Color` (sRGB u8 con alfa directo, `hex`, `mix` como `color-mix`, `with_alpha`, conversión a lineal) y `TextId { widget, part }` (contenido, valor o placeholder de un elemento).
- `Quad { rect, fill, border, border_width, radius, blur }`: una caja con esquinas redondeadas, borde interior y desenfoque opcional; sirve para fondos, bordes, sombras, anillos de foco, caret, selección y scrollbar.
- `DrawOp { Quad, PushClip, PopClip, Text { id, origin, color } }` y `DrawList` con `clear` (color de fondo de la raíz).
- `resolve(&DrawList) -> Vec<Batch>` resuelve la pila de clips en CPU y agrupa las operaciones en lotes alternos `Quads`/`Texts` **en orden de pintor**: un texto pintado antes que una caja queda debajo de ella. Descarta lo totalmente recortado o transparente.

### Paint (`render/paint.rs`)

`paint(&PaintInput { layout, theme, focused, text_fields }) -> DrawList` es puro. Recorre el árbol en `paint_order` (z-index) y, por nodo, pinta en el orden de CSS:

1. `box-shadow` (quad desenfocado, desplazado y expandido).
2. `ring` de foco por fuera del borde, con `ring-offset`.
3. Fondo y borde: con borde uniforme se usa un único quad SDF con radio; con lados distintos (`border-b`, `border-t`…), un rectángulo por lado.
4. Texto, o el contenido del input (selección, valor o placeholder y caret), recortado a su interior.
5. Hijos, dentro del clip del nodo si tiene `overflow`.
6. Barra de scroll superpuesta (4 pt, `muted-foreground` translúcido) cuando hay overflow.

La opacidad efectiva multiplica el alfa de todo lo que pinta el nodo. No hay colores literales: todo sale del `Style` resuelto, que a su vez sale del tema.

### Backend (`render/wgpu_backend.rs` y `quad.wgsl`)

- `WgpuRenderer` encapsula device, pipeline, glyphon y la superficie, o una textura offscreen `Rgba8UnormSrgb` (`new_offscreen` + `render_to_rgba`, usado por las capturas).
- Quads instanciados: cada instancia lleva rect, relleno, borde, clip y `(radio, ancho de borde, blur)`. El shader evalúa la SDF de un rectángulo redondeado, descarta fuera del clip, suaviza el borde con antialiasing y aplica el blur con `smoothstep`. La mezcla es en alfa premultiplicado. El buffer de instancias crece a la siguiente potencia de dos.
- Cada lote de texto tiene su propio `TextRenderer` de glyphon (todos comparten el atlas), así los textos se intercalan con las cajas en el orden correcto. Los `TextBounds` salen del clip resuelto en píxeles físicos.
- Los colores se convierten a lineal cuando el formato es sRGB, igual que glyphon. `render` devuelve `Ok(false)` cuando hay que redibujar (superficie perdida u obsoleta).

### Texto (`render/text.rs` y `text_field.rs`)

- `TextStore` guarda un buffer cosmic-text por `TextId`; `sync_text` solo re-modela si cambian el texto o el estilo e implementa `TextMeasure`. Salto solo entre palabras (`Wrap::Word`); `whitespace-nowrap` usa `Wrap::None`.
- El input es de una línea (`Wrap::None`) con `scroll_x` propio, porque glyphon ignora el scroll horizontal del buffer. El preedit IME se muestra subrayado sin modificar el valor. Las métricas del editor siguen al estilo resuelto del input.

### Capturas

`cargo test --manifest-path desktop/Cargo.toml -- --ignored snapshot_demo` renderiza la demo offscreen en la GPU local (2×) y escribe BMP en `desktop/target/snapshots/`: reposo, interacción (hover, foco y valor), ventana estrecha, y scroll al final con anillo de foco. Para verlas como PNG: `sips -s format png demo-idle.bmp --out demo-idle.png`. Es `#[ignore]` porque necesita GPU.

### Pendiente

- [ ] Comparación automática de capturas contra referencias (requiere un runner con GPU y tolerancia de rasterización).
- [ ] Sombras múltiples, gradientes, transformaciones e imágenes/iconos.
- [ ] Política de recursos de fuentes (carga propia, invalidación al cambiar de escala) más allá del `FontSystem` del sistema.

## Accesibilidad

Ver [08-accesibilidad.md](08-accesibilidad.md): el árbol AccessKit se genera desde el layout del árbol `Element`. Contexto original:

Investigar `accesskit` y el puente con la API de accesibilidad de macOS desde el prototipo. Los widgets dibujados a mano deben exponer roles, nombres, estados, acciones, foco y límites. Si el puente no cubre lo requerido, es un riesgo bloqueante que debe resolverse antes del lanzamiento, no una tarea cosmética. Estado actual en [08-accesibilidad.md](08-accesibilidad.md).

## Criterios de aceptación

- Renderer presenta primitivas y texto sin fugas ni fallos al resize/cierre.
- Coordenadas y escala coinciden con el layout y el hit-testing.
- Edición de texto y entrada IME tienen una estrategia documentada y probada.
- No se depende de `egui`/`iced` para renderizar widgets de producción.
