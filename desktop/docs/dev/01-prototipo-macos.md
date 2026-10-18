# 01 — Prototipo vertical para macOS

## Objetivo

Validar el mayor riesgo temprano: dibujar y hacer interactivos controles propios en una ventana de escritorio macOS. Este prototipo no pretende ser todavía la API pública de Kai.

## Preparación

1. Instalar/verificar Rust estable con `rustup`, Cargo y Xcode Command Line Tools.
2. Crear un binario de prueba bajo `desktop/` y registrar las versiones del toolchain y macOS usadas.
3. Añadir solo dependencias necesarias para abrir una ventana y presentar un frame. Evaluar `winit` y `wgpu`; fijar versiones después de comprobar sus ejemplos y compatibilidad.
4. Confirmar ejecución en el Mac de desarrollo y anotar cualquier requisito de permisos o configuración de macOS.

## Estado de implementación

- [x] Crate binario inicial en `desktop/` con `winit` y `wgpu`.
- [x] Ventana macOS redimensionable, cierre y presentación de un color de fondo.
- [x] Backend verificado en Apple Silicon: Metal / Apple M4.
- [x] Geometría visible, rectángulos en unidades lógicas, hit-testing compartido y conversión a NDC.
- [x] Conversión de puntero físico a lógico y actualización de escala de pantalla; ejecución confirmó escala `2.00x`.
- [x] Texto estático dibujado con `glyphon` sobre `wgpu`.
- [x] Botón propio con estados hover/pressed/focus y activación por mouse, Enter o Espacio; Tab alterna el foco.
- [x] La activación actualiza el estado y el mensaje visible.
- [x] Campo editable con cursor, selección, navegación, borrado e inserción desde texto de teclado (`KeyEvent.text`).
- [x] Eventos de preedición/commit IME conectados y portapapeles Cmd+A/C/X/V implementado.
- [x] Validación manual reportada: escritura de texto normal, acentos y recorrido de foco con Tab.
- [x] Validación manual reportada como correcta de IME japonés: al escribir `mihongo` y pulsar Espacio aparecen opciones de candidatos y se puede escribir en japonés.
- [x] Validación manual del portapapeles reportada por el usuario.
- [x] Validación manual reportada: uso de acentos, foco correcto y cambio de la app entre pantallas.
- [x] Revisión visual manual de la app tras la extracción L0; el usuario confirmó que está correcta.

La primera versión del campo no consumía `KeyEvent.text`; ese camino ya está conectado. El usuario confirmó que la escritura normal y los acentos funcionan, que el foco es correcto, que el portapapeles funciona, que la app funciona al cambiar entre pantallas y que la revisión visual después de la extracción L0 está correcta. También reportó correcta la prueba IME japonesa: escribir `mihongo` y pulsar Espacio muestra candidatos y permite escribir en japonés. No se registraron por separado los pasos de commit con Enter y cancelación con Escape.

Para ejecutar el estado actual, consultar [desktop/README.md](../../README.md).

## Implementación en pasos

1. Crear una ventana y responder al cierre y resize.
2. Limpiar la superficie con un color sólido usando el renderer de prueba.
3. Dibujar formas simples (rectángulos y bordes) y verificar escalado Retina/HiDPI.
4. Dibujar texto con una solución candidata; comprobar varias fuentes y tamaños.
5. Dibujar un botón Kai y detectar hit-test/click; mostrar hover, pressed y disabled.
6. Añadir un campo editable: inserción, borrado, selección, movimiento del cursor, teclado y portapapeles.
7. Investigar entrada de texto macOS e IME —incluyendo acentos y composición—; no implementar un `TextField` confiando solo en key-down.
8. Probar redimensionado, pérdida/restauración de foco, cambio de escala y cierre repetido.
9. Registrar latencia percibida, errores, APIs faltantes, limitaciones de accesibilidad y deuda técnica.

## Pruebas manuales mínimas

- Abrir y cerrar la ventana sin panic.
- Redimensionar y mover la ventana mientras se actualiza el contenido.
- Click, click sostenido, movimiento fuera/dentro y pérdida de foco en el botón.
- Escribir, seleccionar, copiar, cortar, pegar y editar caracteres acentuados en el campo.
- Probar un método IME con preedición y commit; verificar que el texto no se duplique y que el candidato aparezca junto al cursor.
- Confirmar que Tab y Shift+Tab recorren los controles en el orden esperado y que Enter/Espacio activan el botón cuando tiene foco.
- Probar al menos una pantalla Retina; comparar puntos lógicos y píxeles físicos.
- Confirmar que el render no ejecuta trabajo bloqueante de red/IO.

## Validación manual pendiente del prototipo

- [x] Validación manual reportada como correcta de composición IME japonesa y candidatos: escribir `mihongo` y pulsar Espacio muestra opciones y permite escribir en japonés.
- [x] Probar acentos; el usuario confirmó que funcionan. La matriz completa de dead keys/caracteres queda pendiente si se requiere cobertura exhaustiva.
- [x] Validar el portapapeles real; el usuario confirmó que funciona.
- [x] Validar que el foco es correcto; completar la comprobación explícita de `Tab`/`Shift+Tab` y cursor durante composición IME si no se hizo en la sesión manual.
- [x] Confirmar funcionamiento al cambiar la app entre pantallas y revisión visual correcta tras la extracción L0; el usuario volvió a confirmar que el cambio de pantalla funciona.
- [ ] Registrar por separado commit con Enter y cancelación con Escape si se necesita cobertura detallada; una inspección instrumental de Retina/cursor también queda pendiente si se requiere.
- [x] Reprobar el cambio de pantalla tras el `SIGTRAP` histórico: el usuario informa que ahora funciona correctamente. La causa del trap previo no se identificó ni se cambió código para corregirla; volver a investigar si reaparece.

## Decisión de continuidad

Se continúa con la siguiente fase porque el prototipo validó el riesgo principal del stack en macOS: render, input básico, foco, texto, acentos, portapapeles, composición IME japonesa con candidatos y cambio de pantalla. El conjunto base de `winit` + `wgpu` + `glyphon`/`cosmic-text` demuestra ser viable para abrir una ventana, dibujar controles propios y manejar interacciones clave en un entorno de escritorio real. Hubo un `SIGTRAP` histórico al cambiar de escala; la última repetición manual fue correcta, pero no se identificó su causa ni se aplicó un arreglo específico.

La decisión es continuar con la arquitectura del desktop y separar la base en capas estables: layout, eventos, render y gestión de foco/text input. No se introduce todavía una API pública ni una abstracción general del framework; el objetivo sigue siendo validar que el stack pueda sostener las necesidades fundamentales de Kai en macOS.

La implementación arquitectónica seguirá L0–L6, comenzando por L0 y avanzando con un gate verificable por fase. La definición detallada se mantiene en [00-vision-y-arquitectura.md](00-vision-y-arquitectura.md) y [README.md](README.md).

## Salida de la fase

- Prototipo ejecutable y pasos reproducibles para lanzarlo.
- Registro de decisiones: crate y versiones evaluadas, backend GPU activo, estrategia de texto/IME y limitaciones conocidas.
- Decisión explícita de continuar, cambiar dependencias o reducir alcance.

## No hacer todavía

No crear un framework de señales, API genérica de comandos, múltiples backends, persistencia de settings ni micro-optimizaciones. El objetivo es averiguar si el stack puede satisfacer las necesidades fundamentales de Kai en macOS.
