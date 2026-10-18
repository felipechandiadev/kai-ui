# 05 — Widgets y tema Kai

**Fase:** L6 (frontend y árbol de widgets reactivos). No iniciar la migración de widgets hasta que estado/comandos, layout y paint tengan contratos verificables.

## Objetivo

Crear una API declarativa de widgets propios y trasladar selectivamente el lenguaje visual de Kai desde la librería web, conservando una presentación coherente sin copiar literalmente CSS/Tailwind.

## Orden de implementación

1. Definir el árbol mínimo: identidad estable, hijos, estado/props y vínculo con bounds de L4; evitar API pública prematura.
2. **Theme:** colores semánticos, tipografía, escala de espaciado, radios, bordes, alturas y densidad; tokens web son referencia, no runtime.
3. Migrar **Text** y **Button** al flujo árbol → comandos/estado → layout → paint.
4. Migrar **TextField** preservando selección, cursor, teclado, IME y portapapeles a través del adaptador de plataforma.
5. Verificar el ciclo integrado con app demo y pruebas antes de ampliar el catálogo.
6. Incorporar luego Select/AutoComplete y Dialog/Alert/Toast, atendiendo accesibilidad, foco, clipping y acciones.
7. Después: Card, Skeleton, LoadingState, Calendar y DataGrid según prioridad de las apps desktop.

## API y composición

- Definir props Rust tipadas y evitar una API que copie atributos HTML.
- Los widgets describen estructura, estado visual y mensajes; no llaman directamente a servicios ni leen/escriben archivos.
- Los componentes compuestos se construyen con primitivas y layout, salvo que un control necesite un tratamiento especializado.
- Mantener tema y estado de interacción fuera de valores de color codificados en cada componente.
- Definir una ruta para iconos/SVG y verificar licencias; no aceptar cualquier SVG sin política de seguridad y límites de complejidad.
- Si se requieren docking, tabs o toolbar overflow, evaluar las piezas de `craft-ui` por separado; consultar [07-integracion-craft-libs.md](07-integracion-craft-libs.md) y no adoptar su renderer egui como árbol visual de Kai.

## Traslado desde Kai Web

Consultar `web/src/theme/tokens.css`, los README de componentes y `web/DESIGN-SYSTEM.md` para recuperar intención, variantes, estados y semántica. Mapear cada token a unidades y capacidades reales del renderer. Marcar explícitamente los tokens sin equivalente o que requieran adaptación.

## Criterios de aceptación

- Cada widget inicial tiene documentación de props, estados, teclado, accesibilidad y ejemplos.
- Variantes visuales derivan del tema; cambios de tema no requieren editar todos los widgets.
- Foco y teclado son utilizables sin mouse.
- Las pruebas cubren transición de estados, mensajes emitidos y geometría interactiva.
