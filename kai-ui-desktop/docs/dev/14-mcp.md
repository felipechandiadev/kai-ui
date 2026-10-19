# 14 — MCP experimental: cómo probarlo

**Tipo:** guía práctica. El diseño y las decisiones están en los Pasos 12 y 13 de [13](13-hoja-de-ruta-plataforma.md) (§7.3, §7.4 y §12). Aquí se explica cómo compilar la demo como servidor MCP, conectarla a Cursor o a Claude Desktop y qué revisar.

> **Experimental.** Solo stdio: el cliente lanza la app con `--mcp` y la app termina cuando el cliente se desconecta. No hay puente hacia una ventana ya abierta (Paso 24). Las acciones destructivas piden confirmación en la ventana, y cada llamada queda en una auditoría en memoria (Paso 13).

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

Para ver la confirmación, se agrega `{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"contador.reiniciar","arguments":{}}}` después de la suma y se alarga el `sleep` a 40 s. El diálogo aparece en la ventana. Si nadie lo responde, a los 30 s llega un error de tiempo agotado y el diálogo se cierra. Con `{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":4}}` se cierra antes, sin respuesta.

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
| `contador.reiniciar` | acción destructiva | — | Pide confirmación; luego «Hecho», o error si la persona rechaza o si el contador ya está en cero |
| `form.enviar` | acción | — | `{ "result": nombre }`, o error «Falta el nombre» |
| `form.limpiar` | acción | — | «Hecho» |
| `form.ultimo_envio` | consulta | — | `{ "result": nombre o null }` |

`form.alternar_contrasena` no está expuesta: no se lista, y si un agente la llama igual recibe «Herramienta desconocida». La lista sale del registro (`.expose()`) y coincide con los comandos que tienen `"exposed": true` en `mcp-schema.json`. `contador.reiniciar` es la única acción `Destructive` y lleva `destructiveHint`; las demás son `Mutating`, y las consultas llevan `readOnlyHint`.

## 4. Confirmación, indicador y plazo

Cuando un agente pide `contador.reiniciar`, la ventana muestra el diálogo «¿Permitir «Reiniciar»?». El diálogo dice qué cliente lo pide, el id, la descripción y los datos (los campos `writeOnly` salen como `•••`). Tiene tres botones:

- **Permitir en esta sesión:** corre ahora, y el resto de la sesión ese comando ya no pregunta.
- **Permitir una vez:** corre esta vez.
- **Rechazar:** el foco empieza aquí, y Esc hace lo mismo. El agente recibe «La persona rechazó «Reiniciar»» como resultado con error, y el modelo no cambia.

Mientras el diálogo está abierto, los clics fuera de él, los atajos y el menú no hacen nada, y las demás llamadas del agente esperan en cola. Si el contador ya está en cero, `contador.reiniciar` falla sin preguntar. Lo que la persona hace en la ventana (el botón Reiniciar, ⌘0) nunca pregunta.

**Indicador:** al conectarse el cliente, en la esquina inferior derecha aparece «Agente: …» con su nombre (el `title` o el `name` que manda en `initialize`; Cursor, por ejemplo, puede presentarse como `cursor-vscode`). Tras cada acción que corre, se agrega su título: «Agente: … · Sumar».

**Plazo y cancelación:** si la persona no responde en 30 s, el agente recibe «contador.reiniciar no terminó en 30 s (¿esperaba confirmación?)» y el diálogo se cierra sin ejecutar nada. Lo mismo pasa si el cliente cancela la llamada (`notifications/cancelled`). En los dos casos la auditoría la registra como cancelada.

**Auditoría:** cada acción pedida por JSON queda registrada en memoria (las últimas 500), con su origen (agente con su cliente, o script), la entrada redactada, el resultado y la duración. Las rechazadas también. Por ahora solo se lee desde los tests (`TestApp::audit_log`).

## 5. Qué revisar

- **Lista:** el cliente muestra las seis herramientas con su título; todas menos `form.ultimo_envio` traen descripción.
- **Acción reflejada:** pedir «suma 3 al contador». La ventana muestra el nuevo valor sin tocarla.
- **Acción no disponible:** con el contador en cero, pedir que lo reinicie. El agente recibe «El contador ya está en cero» como resultado con error, no como fallo del servidor.
- **Respuesta estructurada:** escribir un nombre en la ventana y pedir «envía el formulario». El agente recibe el nombre.
- **Indicador:** al conectarse, la esquina muestra el nombre del cliente, y después la última acción.
- **Confirmación:** subir el contador y pedir «reinicia el contador». Rechazar: el agente lo informa y el contador no cambia. Pedirlo de nuevo y elegir «Permitir una vez»: se reinicia. Pedirlo una tercera vez con «Permitir en esta sesión», y una cuarta: esta ya no pregunta.
- **Plazo:** pedir un reinicio y no responder. A los 30 s el diálogo se cierra, y el agente recibe el error de tiempo agotado. Ver también si el cliente cancela antes por su cuenta.
- **Lector de pantalla:** con VoiceOver, el diálogo se anuncia como diálogo con su título, y el foco queda en Rechazar.
- **Salida limpia:** al desactivar el servidor o cerrar el cliente, la ventana se cierra sola.

## 6. Riesgos conocidos

- **Nombres con punto.** El spec MCP los admite, pero algunos clientes o modelos piden nombres con solo letras, dígitos, `_` y `-`. Si un cliente los rechaza o los renombra, se agrega el alias `contador_sumar` (§12 de 13).
- **Una ventana por cliente.** Cada cliente lanza su propia instancia de la app, sin estado compartido.
- **Plazo.** Una llamada que no responde en 30 s vuelve como error de tiempo agotado, también mientras espera confirmación. Puede ser poco para leer el diálogo con calma. Si molesta, el plazo se hace configurable o se descuenta el tiempo de confirmación.
- **Clientes que también confirman.** Cursor y Claude Desktop pueden pedir su propia aprobación antes de llamar una herramienta con `destructiveHint`, así que la persona confirma dos veces.
