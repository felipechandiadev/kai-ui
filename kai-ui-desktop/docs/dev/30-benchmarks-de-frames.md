# 30 · Benchmarks de frames

Tarea #11, parte 1: medir antes de optimizar. Los benchmarks están en
`demo/benches/frames.rs` y usan [criterion](https://criterion-rs.github.io/book/)
(dependencia de desarrollo de la demo, sin `rayon` ni gráficos). No abren
ventana ni usan la GPU: corren sobre `TestApp` con texto real
(`real_text()`), así que sirven en macOS, Linux y CI.

## 1. Qué mide

| Grupo | Qué hace cada iteración | Secciones / tamaños |
|---|---|---|
| `element_map/view/<sección>` | `view()` de la raíz: construir el árbol | `home`, `fields`, `table`, `map`, `catalog` |
| `element_map/map/<sección>` | `Element::map` sobre ese árbol (lo que cuesta envolverlo una vez más, como un layout alrededor de una página) | ídem |
| `layout/<sección>` | Un resize entre 1024 y 800 pt: la sesión vuelve a construir la vista y la distribuye (taffy, texto real), sin pintar | ídem |
| `paint/<sección>` | La lista de dibujo de un frame ya distribuido | ídem |
| `layout_paint/<sección>` | `layout` + `paint`: un frame completo sin GPU | ídem |
| `rebuild_large_tree/<filas>` | Un mensaje (Espacio sobre un botón) en una lista no virtual de 250, 1000 y 4000 filas (4 elementos por fila): `update`, vista, layout y pintura | 250, 1000, 4000 |

`demo/tests/benches.rs` sigue igual: son pruebas `#[ignore]` con
percentiles impresos (y el tope de 4 ms del pan del mapa). Los de criterion
agregan estadística, detección de cambios y baselines.

## 2. Cómo correrlos

Desde `kai-ui-desktop/demo`:

```sh
cargo +1.95 bench --bench frames                       # todo (~2-3 min)
cargo +1.95 bench --bench frames -- layout_paint       # un grupo
cargo +1.95 bench --bench frames -- '^paint/'          # el filtro es una regex
cargo +1.95 bench --bench frames -- rebuild_large_tree/1000
```

Para una pasada rápida (menos precisa):

```sh
cargo +1.95 bench --bench frames -- --warm-up-time 0.5 --measurement-time 2
```

Criterion imprime `time: [bajo medio alto]` (intervalo de confianza del
tiempo por iteración) y, si hay una medición anterior, el cambio y si es
significativo (`p < 0.05`). Los resultados quedan en
`target/criterion/<grupo>/<nombre>/`.

## 3. Baselines

Una baseline es una medición con nombre para comparar después:

```sh
# En main, antes del cambio:
cargo +1.95 bench --bench frames -- --save-baseline main
# Con el cambio:
cargo +1.95 bench --bench frames -- --baseline main
```

`--baseline main` compara sin sobrescribirla; `--save-baseline` la
reemplaza. Reglas para que la comparación valga:

- Misma máquina, enchufada a corriente, sin otras compilaciones ni la demo
  abierta. En una MacBook, el modo de bajo consumo cambia los números.
- Siempre en release (`cargo bench` ya usa el perfil `bench`).
- Un cambio menor a ~3 % suele ser ruido aunque criterion lo marque.
- La baseline vive en `target/`: no se versiona. Para registrar una
  referencia, anota en el PR la máquina y la tabla de `time:`.

## 4. Referencia (Linux, no es baseline)

Primera pasada en una VM Linux x86_64 con `--warm-up-time 0.3
--measurement-time 1`, solo para tener el orden de magnitud. En una Mac
los números serán otros: tomar la baseline allí.

| Medición | Tiempo medio |
|---|---|
| `element_map/view/home` · `map/home` | 130 µs · 27 µs |
| `element_map/view/catalog` · `map/catalog` | 323 µs · 54 µs |
| `layout/home` · `layout/map` | 0,83 ms · 2,7 ms |
| `paint/home` · `paint/table` | 5,7 µs · 17 µs |
| `layout_paint/table` | 2,1 ms |
| `rebuild_large_tree/250` · `/1000` · `/4000` | 4,3 ms · 19 ms · 84 ms |

Lo que ya se ve, sin optimizar nada: la pintura es una fracción mínima del
frame; casi todo es vista + layout, y el árbol grande crece de forma
lineal (≈ 17–21 µs por fila). Es el punto de partida de la parte 2 de la
tarea #11 (layout incremental en vez de `TaffyTree::new` en cada frame).
