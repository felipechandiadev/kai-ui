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

## Accesibilidad

Investigar `accesskit` y el puente con la API de accesibilidad de macOS desde el prototipo. Los widgets dibujados a mano deben exponer roles, nombres, estados, acciones, foco y límites. Si el puente no cubre lo requerido, es un riesgo bloqueante que debe resolverse antes del lanzamiento, no una tarea cosmética.

## Criterios de aceptación

- Renderer presenta primitivas y texto sin fugas ni fallos al resize/cierre.
- Coordenadas y escala coinciden con el layout y el hit-testing.
- Edición de texto y entrada IME tienen una estrategia documentada y probada.
- No se depende de `egui`/`iced` para renderizar widgets de producción.
