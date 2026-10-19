# 14 — MCP experimental: cómo probarlo

**Tipo:** guía práctica. El diseño y las decisiones están en el Paso 12 de [13](13-hoja-de-ruta-plataforma.md) (§7.3, §7.4 y §12). Aquí se explica cómo compilar la demo como servidor MCP, conectarla a Cursor o a Claude Desktop y qué revisar.

> **Experimental.** Solo stdio: el cliente lanza la app con `--mcp` y la app termina cuando el cliente se desconecta. No hay confirmación de acciones destructivas ni auditoría (Paso 13), ni puente hacia una ventana ya abierta (Paso 24).

## 1. Compilar

```sh
cd kai-ui-desktop
cargo build --features mcp
```

El binario queda en `target/debug/kai-ui-desktop-prototype`. Sin la feature, `--mcp` termina con «--mcp necesita compilar con --features mcp».

Antes de configurar un cliente se puede probar a mano. Se mandan líneas JSON-RPC por stdin; las respuestas salen por stdout y los registros de la app por stderr:

```sh
( printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"prueba","version":"0"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"contador.sumar","arguments":{"cantidad":5}}}'
  sleep 3 ) | target/debug/kai-ui-desktop-prototype --mcp 2>/dev/null
```

La ventana se abre, el contador sube a 5 y, cuando `sleep` termina y se cierra stdin, la app sale con código 0.

## 2. Configurar el cliente

La misma entrada sirve en Cursor (`~/.cursor/mcp.json`, o `.cursor/mcp.json` en el proyecto) y en Claude Desktop (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "kai-demo": {
      "command": "/Users/felipe/dev/kai-ui/kai-ui-desktop/target/debug/kai-ui-desktop-prototype",
      "args": ["--mcp"]
    }
  }
}
```

La ruta debe ser absoluta. Las variables de la demo (`KAI_THEME`, `KAI_BG`…) se pasan con `"env": { "KAI_THEME": "appkit" }`. Después de recompilar hay que reiniciar el servidor desde el cliente, porque el proceso viejo sigue abierto.

## 3. Qué ve el agente

| Herramienta | Tipo | Entrada | Respuesta |
|---|---|---|---|
| `contador.incrementar` | acción | — | «Hecho» |
| `contador.sumar` | acción | `{ "cantidad": u32 }` | «Hecho» |
| `contador.reiniciar` | acción | — | «Hecho», o error si el contador ya está en cero |
| `form.enviar` | acción | — | `{ "result": nombre }`, o error «Falta el nombre» |
| `form.limpiar` | acción | — | «Hecho» |
| `form.ultimo_envio` | consulta | — | `{ "result": nombre o null }` |

`form.alternar_contrasena` no está expuesta: no se lista, y si un agente la llama igual recibe «Herramienta desconocida». La lista sale del registro (`.expose()`) y coincide con los comandos que tienen `"exposed": true` en `mcp-schema.json`. Las acciones de la demo son todas `Mutating`; una `Destructive` llevaría `destructiveHint`, y las consultas llevan `readOnlyHint`.

## 4. Qué revisar

- **Lista:** el cliente muestra las seis herramientas con su título; todas menos `form.ultimo_envio` traen descripción.
- **Acción reflejada:** pedir «suma 3 al contador». La ventana muestra el nuevo valor sin tocarla.
- **Acción no disponible:** con el contador en cero, pedir que lo reinicie. El agente recibe «El contador ya está en cero» como resultado con error, no como fallo del servidor.
- **Respuesta estructurada:** escribir un nombre en la ventana y pedir «envía el formulario». El agente recibe el nombre.
- **Salida limpia:** al desactivar el servidor o cerrar el cliente, la ventana se cierra sola.

## 5. Riesgos conocidos

- **Nombres con punto.** El spec MCP los admite, pero algunos clientes o modelos piden nombres con solo letras, dígitos, `_` y `-`. Si un cliente los rechaza o los renombra, se agrega el alias `contador_sumar` (§12 de 13).
- **Una ventana por cliente.** Cada cliente lanza su propia instancia de la app, sin estado compartido.
- **Plazo.** Una llamada que no responde en 30 s vuelve como error de tiempo agotado.
