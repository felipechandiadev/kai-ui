# 02 — Geometría y layout

**Fases:** L0 (base geométrica) y L4 (maquetación incremental/caché). Implementar L0 y validarlo antes de L1; L4 se inicia tras L1 y L2–L3.

## Estado actual de L0

- [x] Módulo geométrico puro inicial en `desktop/src/geometry.rs`, con `Point`, `Size`, `Rect`, `Insets`, `PhysicalPoint` y `ScaleFactor`.
- [x] Hit-testing, conversión lógico/físico y transformación de rectángulos a NDC comparten esas primitivas.
- [x] Tests de bordes, rectángulos inválidos/vacíos, intersección, traslación, insets y escala Retina.
- [x] Verificación visual manual de render e interacción tras la extracción, ejecutando la app en macOS; el usuario confirmó que está correcta.
- [ ] Revisar si las operaciones geométricas adicionales requeridas por layout justifican otro tipo/dependencia; no añadir una crate geométrica por anticipado.

La extracción L0 queda aceptada: las pruebas automáticas pasan y la revisión visual manual fue confirmada. IME y portapapeles mantienen su estado de validación independiente de las capas geométricas.

## Objetivo

Definir coordenadas coherentes entre ventana, layout, hit-testing y renderer; después habilitar composiciones básicas para los widgets.

## Pasos

1. **L0:** documentar espacio y unidades: puntos lógicos, origen, eje Y, límites y conversión a píxeles físicos por escala de ventana.
2. **L0:** consolidar tipos mínimos `Point`, `Size`, `Rect`, `Insets` y `Scale` (o equivalente); elegir tipos propios inicialmente y comparar `kurbo`/`euclid` solo si una necesidad real lo justifica.
3. **L0:** especificar y probar intersección, contención, traslación y conversión lógico/físico; definir semántica de tamaños no positivos, valores no finitos y rectángulos vacíos.
4. **L0:** migrar el hit-testing y las cajas de presentación para que render e interacción usen la misma geometría.
5. **Gate L0:** tests unitarios deterministas de geometría, bordes, valores límite, escala Retina y transformación a NDC; la app debe conservar comportamiento.
6. **L4:** usar CSS Grid como modelo principal de composición, con tracks, auto-placement, gap, padding, alineación y restricciones explícitas. Taffy es el algoritmo candidato para aproximar la semántica CSS.
7. **L4:** mantener el layout desacoplado del renderer; compartir resultados entre render y hit-testing, y probar árbol conocido, resize y escala.
8. **L4:** empezar con layout completo correcto; añadir invalidación/caché solo tras benchmark reproducible y pruebas de paridad frente a recomputación completa.

## Contrato esperado

El layout calcula cajas; no dibuja ni procesa eventos. El renderer consume las cajas. El sistema de interacción pregunta a esas mismas cajas qué nodo está bajo el puntero. Así se evita que la apariencia y los clics diverjan.

## Comportamiento responsive del modelo de cajas

- La raíz `WindowRootLayout` mide exactamente el viewport lógico de la ventana; es el padre de `MainContentLayout`, que contiene un `ScrollViewportLayout`, y este contiene las cajas de los controles.
- Cada contenedor define una caja visible y un área de contenido con padding. Los hijos reciben el ancho disponible dentro de esa área.
- CSS Grid es el modelo de composición principal: los hijos se colocan en filas/columnas, los tracks distribuyen espacio disponible, y el auto-placement avanza en orden de fila. No se usa una regla horizontal manual tipo flex como contrato principal.
- El tamaño de un hijo sin dimensión explícita se resuelve por las tracks y alineación del grid padre; las restricciones mínimas y la medición intrínseca del widget son parte del contrato, no valores adivinados por el renderer.
- El texto se compone al ancho de su caja: al reducir el ancho, `cosmic-text` lo envuelve en más líneas y el layout usa la altura medida para colocar los siguientes hijos.
- La reducción de altura no escala ni comprime controles, tipografía ni gaps. El panel mantiene un viewport; si el contenido natural lo excede, `scroll_range` habilita desplazamiento vertical.
- La rueda/trackpad desplaza el contenido dentro de ese viewport. Render, hit-testing, selección, caret e IME comparten el mismo `scroll_offset` y las mismas cajas.
- El prototipo establece un mínimo de ventana de **420×340 puntos lógicos**. Kai implementa el subconjunto explícito de CSS requerido; el objetivo no es incorporar un parser/renderer web.

```text
WindowRootLayout (bounds = tamaño lógico completo de la ventana)
└── MainContentLayout (panel y padding)
	└── ScrollViewportLayout (área visible + content_size + scroll_range)
		└── ContentChildrenLayout
			├── heading_box
			├── description_box
			├── button / button_label_box
			├── status_box
			└── input
```

## Criterios de aceptación

- Tests deterministas para geometría, transforms, escala y cajas vacías.
- Layout reproducible para un conjunto fijo de restricciones.
- Resize actualiza layout y hit-testing de forma consistente.
- No hay promesa de latencia fija sin benchmark documentado.
- L0 no depende de `winit` ni de `wgpu`; la conversión de plataforma ocurre en límites explícitos.
- La ruta con caché produce los mismos bounds que el layout completo para los casos de prueba.

## Estado actual de L4

- [x] `desktop/src/layout.rs` implementa `DemoLayout::compute` como cálculo completo y determinista sobre un viewport en unidades lógicas; las cajas del contenido se colocan usando CSS Grid vía Taffy.
- [x] El resultado centraliza bounds del panel, botón y campo, además de orígenes de texto y caret; hit-testing, dibujo, selección e IME usan el mismo resultado.
- [x] El viewport de referencia conserva las coordenadas visuales originales; viewports más estrechos reducen los anchos disponibles.
- [x] Texto de título, descripción y estado se mide con `cosmic-text` al ancho de su caja; sus buffers envuelven líneas y el flujo vertical usa alturas medidas.
- [x] El panel crece con el contenido envuelto cuando hay espacio disponible; los tracks de fila del Grid reciben las alturas medidas y los gaps existentes. Si el viewport vertical es menor, conserva los tracks de controles/gaps y expone rango de scroll.
- [x] El scroll vertical actualiza dibujo, clipping, hit-testing, selección y caret/IME; hay indicador visual de scrollbar cuando existe overflow.
- [x] Tamaño mínimo de ventana: 420×340 puntos lógicos. Las pruebas cubren reflow por altura medida, scroll range, límites y conversión viewport/contenido.
- [x] La raíz de ventana queda representada explícitamente en el layout; el contenido principal, viewport y cajas de hijos se anidan como nodos estructurales, no solo como coordenadas planas.
- [x] Consumer spike de `taffy` 0.15 con solo `std`, `taffy_tree` y `grid`: tracks `fr`, auto-placement por filas, gap y cálculo en unidades lógicas comprobados con tests.
- [x] Migración inicial de `DemoLayout` a una columna CSS Grid con filas explícitas; las alturas de texto medidas por `cosmic-text` y las alturas/gaps de controles alimentan tracks lógicos.
- [ ] Evolucionar a definición declarativa de tracks/áreas y medida intrínseca por nodo; esta migración inicial mantiene las medidas preferidas existentes para conservar el comportamiento observable.
- [ ] Evaluar restricciones intrínsecas/min-content, áreas nombradas y política de overflow frente a los casos requeridos por Kai antes de estabilizar una API propia.
- [x] Verificación visual manual del primer layout compartido reportada correcta por el usuario.
- [ ] Revisar visualmente el reflow de texto al reducir ancho y el scroll al reducir altura hasta el mínimo admitido.
- [ ] Repetir la comprobación específica de mover la ventana a una segunda pantalla; no se realizó en esta ronda de pruebas L4.
- [ ] Completar el consumer spike con medidas intrínsecas, restricciones mínimas, áreas y overflow antes de declarar cerrada la evaluación/adopción de `taffy`.
- [ ] No hay caché ni invalidación incremental todavía; medir antes de añadirlas.

Este es un incremento inicial de L4, no el motor general de layout completo del framework. `DemoLayout` ahora obtiene las posiciones y cajas principales de una columna CSS Grid con filas explícitas; el panel exterior y su viewport siguen siendo responsabilidad del adaptador de demo. La integración aún fija tracks de controles y gaps desde las medidas actuales, por lo que no equivale a una API declarativa CSS completa ni cierra la evaluación de `taffy`. El layout se recalcula íntegramente cuando se solicita; no se reclama una mejora de rendimiento.

La revisión manual del primer incremento L4 queda aprobada para las pruebas ejecutadas. El modelo responsive con wrapping y scroll vertical está implementado y automatizado; requiere inspección visual manual. La validación de segunda pantalla queda pendiente para esta ronda.
