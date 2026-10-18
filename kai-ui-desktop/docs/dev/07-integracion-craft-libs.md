# 07 — Evaluación e integración de craft-libs

## Propósito

Evaluar qué crates de [`storytold/craft-libs`](https://github.com/storytold/craft-libs) aportan valor a Kai Desktop sin cambiar el acuerdo principal: **Kai conserva su árbol de widgets y dibuja su UI con un renderer propio**. La información de esta guía se basa en el README, manifests y documentación pública del repositorio consultados el 17 de octubre de 2026; revisar upstream antes de adoptar una dependencia.

> `craft-libs` es un workspace de bibliotecas para las aplicaciones Craft, no un backend gráfico genérico para Kai. Sus crates tienen alcances distintos. No incorporar el workspace entero ni asumir que todas sus librerías son independientes de egui.

## Clasificación para Kai

| Crate o recurso | Posible uso en Kai Desktop | Acoplamiento / decisión inicial |
|---|---|---|
| `craft-ui` | Algoritmos puros que merezcan reutilizarse: modelo transaccional de docking, cálculo de splits, ajuste de tabs/toolbar, contratos de estados visuales o catálogo SVG como referencia. | **No usar su `DockArea` ni sus widgets como frontend**: su superficie visual está construida sobre egui. El crate documenta egui 0.36 y Rust 1.95+. Evaluar si los módulos/modelos necesarios se pueden consumir sin egui o extraer/adaptar solo una pieza con licencia compatible; no copiar código sin revisar licencia y atribución. |
| `craft-quit` | Referencia para el cierre y menú de macOS; verificar si su función `quit()` sirve con el event loop elegido. | Sus ganchos de cierre y menú están condicionados por `egui`/`eframe`/winit. No asumir que son plug-and-play con un loop `winit` propio. Probar macOS y la política de documentos sin guardar. |
| `craft-cms` | Gestión ICC cuando Kai muestre, convierta o imprima imágenes/documentos con requisitos de color. Puede alimentar transforms/LUTs del renderer. | Opcional y de dominio visual/documental; no es el sistema de colores/tokens de widgets. Incluir solo si existe requisito real de fidelidad de color; revisar perfiles incorporados, limitaciones y políticas de distribución. |
| `craft-atomic` | Guardado robusto de documentos, exports o estado persistente de una app Kai. | Servicio de IO de la aplicación, no dependencia necesaria de `ui-core`/`ui-render`. Sus garantías dependen del filesystem; preservar políticas de permisos y limitaciones de metadata descritas upstream. |
| `craft-settings` | Preferencias de una app desktop: tema, escala, densidad y configuración. | Integración opcional de aplicación; no debe definir el tema ni el estado de widgets de Kai. Depende de `craft-atomic`; evaluar autosave/debounce, backup, migraciones y modo memory para pruebas. |
| `craft-pdf` | Lectura, rasterización o escritura PDF para apps que lo necesiten. | Extensión opcional de aplicación. No es parte del layout ni del renderer UI; revisar features, límites de entrada, procedencia de backends vendorizados y políticas/licencias de fonts/assets. |
| `craft-print` | Descubrimiento y envío de trabajos PDF a CUPS/macOS y otras plataformas soportadas. | Servicio opcional. Generación/layout del PDF, diálogo/previsualización, permisos y UX siguen siendo de la app. Validar el soporte macOS real y ejecutar la operación fuera del hilo de UI. |
| `craft-ai-core` | Contratos de datos para solicitudes/herramientas de IA si una app Kai integra esos flujos. | Fuera del núcleo visual y del MVP. No ejecutar herramientas ni otorgar permisos automáticamente por importar el modelo. |

## Frontera futura MCP / agentes de IA

La integración, si una aplicación la necesita, se ubica fuera de `ui-core`, `ui-render` y del árbol de widgets:

1. Un adaptador opcional de aplicación recibe el mensaje MCP/agente.
2. La aplicación valida identidad, permisos, esquema, contexto y allowlist; la autorización pertenece a la app, no al renderer.
3. El adaptador convierte únicamente operaciones autorizadas a comandos tipados del catálogo lógico `ui-engine` (L2–L3).
4. El dispatcher actualiza el modelo por el flujo unidireccional normal. Resultados y errores vuelven a la app como eventos/estado; no se exponen handles GPU ni se ejecutan herramientas desde el event loop de dibujo.

`ui-engine` es por ahora una frontera conceptual, no una decisión de crate/workspace. `craft-ai-core` puede aportar contratos de datos si hay un caso concreto; no se incorpora como dependencia de L1 ni se considera un servidor MCP o una capa de autorización. La evaluación futura debe fijar revisión upstream y probar MSRV, features, seguridad y comportamiento de la integración.

## Límite especial de `craft-ui`

`craft-ui` puede ser útil como **referencia de diseño de APIs y como fuente candidata de lógica de dominio visual**, pero no satisface directamente el objetivo de un renderer Kai. La documentación upstream describe icon buttons, tabs, resize handles, docking, toolbar y print settings para frontends egui; su `DockArea` dibuja con egui. El modelo de docking puede ser distinto de su renderer y admite snapshots/acciones, pero los tipos/funciones que Kai quiera consumir deben comprobarse frente al manifest y feature flags del commit fijado.

Antes de depender del crate:

1. Inspeccionar el `Cargo.toml` y el módulo concreto para conocer dependencias obligatorias/features; no inferir desacoplamiento solo desde el README.
2. Hacer un pequeño consumer test que demuestre que el módulo escogido compila con el stack Kai y sin activar el frontend egui, si esa es la intención.
3. Si depende inevitablemente de egui, elegir entre aceptar esa dependencia solo para modelo puro, adaptar la idea con una implementación independiente, o no adoptarlo. No conectar egui como un segundo renderer.
4. Mantener la interacción y dibujo propios de Kai: traducir eventos Kai a acciones del modelo cuando corresponda y presentar la disposición con `ui-layout`/`ui-render`.

## Plan de adopción por etapas

### Ahora: prototipo de renderer

- `craft-libs` no bloquea la elección de `winit` + `wgpu`, layout o texto.
- Mantener el prototipo de botón y campo editable independiente de `craft-ui`.
- Consultar `craft-quit` solo al probar cierre, menú y ciclo de vida macOS; registrar si es compatible con el loop real.

### Después de validar el núcleo

- Comparar los requisitos de docking/tabs de Kai con `craft-ui::docking` y helpers geométricos. Probar compatibilidad antes de incorporarlos.
- Añadir `craft-settings` si la primera app necesita persistir preferencias; usar configuración en memoria en tests.
- Incorporar `craft-atomic`, `craft-pdf`, `craft-print`, `craft-cms` o `craft-ai-core` únicamente a través de capacidades opcionales de app y requisitos explícitos.

## Versiones, licencia y supply chain

El repositorio upstream se organiza como workspace Rust y recomienda dependencias Git fijadas a un `rev`. En la revisión consultada, el workspace declara edition 2024, MSRV Rust 1.95, licencia `MIT OR Apache-2.0`; el README de `craft-ui` también indica egui 0.36 y Rust 1.95+. Estos valores pueden cambiar.

Para cada adopción:

1. Fijar una revisión completa e inmutable de Git; evitar `branch = "main"` en builds reproducibles.
2. Confirmar MSRV, target macOS, features y dependencias transitivas con `cargo tree` y una compilación mínima.
3. Mantener aviso de licencia upstream y los notices/atribuciones de assets vendorizados o embebidos. Revisar licencias individuales; la licencia del crate no necesariamente cubre fonts, icons o datos de prueba.
4. Revisar patches `[patch.crates-io]` exigidos por crates que vendorizan o fijan forks, así como sus instrucciones de adopción. Cargo no aplica automáticamente patches declarados solo en el workspace de la dependencia.
5. Documentar la revisión fijada, módulos/features habilitados, justificación, riesgos y ruta de actualización.

## Decisión

`craft-libs` se tratará como **catálogo de dependencias opcionales y referencias de implementación**, no como la base del renderer. El único caso de reutilización visual de alta prioridad potencial es el modelo de docking/layout si los requisitos desktop lo necesitan y su acoplamiento puede aislarse. Todas las demás integraciones quedan condicionadas a un caso de uso de aplicación, con pruebas y revisión de plataforma/licencia.
