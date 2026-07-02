# SDRX Mimic

Editor de notas Markdown para la terminal. Compatible con vaults de Obsidian. Extensible con plugins en Lua y Rhai.

```
  ███████╗██████╗ ██████╗ ██╗  ██╗    ███╗   ███╗██╗███╗   ███╗██╗ ██████╗
  ██╔════╝██╔══██╗██╔══██╗╚██╗██╔╝    ████╗ ████║██║████╗ ████║██║██╔════╝
  ███████╗██║  ██║██████╔╝ ╚███╔╝     ██╔████╔██║██║██╔████╔██║██║██║
  ╚════██║██║  ██║██╔══██╗ ██╔██╗     ██║╚██╔╝██║██║██║╚██╔╝██║██║██║
  ███████║██████╔╝██║  ██║██╔╝ ██╗    ██║ ╚═╝ ██║██║██║ ╚═╝ ██║██║╚██████╗
  ╚══════╝╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝   ╚═╝     ╚═╝╚═╝╚═╝     ╚═╝╚═╝ ╚═════╝
```

---

## Instalación

**Requisitos:** Rust (edición 2021+) · GCC o Clang

```bash
git clone <repo>
cd SDRX-Notes
cargo install --path .
```

El binario `mimic` queda en `~/.cargo/bin/`.

---

## Uso básico

```bash
mimic                                    # abre el último vault o pantalla de inicio
mimic /ruta/al/vault                     # abre vault específico
mimic /ruta/al/vault --new               # crea y abre nuevo vault
mimic /ruta/al/vault --note "texto"      # crea nota rápida (sin abrir TUI)
```

---

## Modos

Mimic opera mediante modos, similar a Vim.

| Modo | Descripción |
|------|-------------|
| **Home** | Pantalla de inicio. Lista vaults recientes |
| **Normal** | Navegación. Modo por defecto al abrir un vault |
| **Insert** | Edición de texto |
| **Command** | Comandos con `:` |
| **Settings** | Configuración: extensiones y temas (`Ctrl+T`) |
| **FileBrowser** | Selector de directorio |
| **NewVaultDialog** | Dialog para crear un nuevo vault |

---

## Keybindings

### Pantalla de inicio (Home)

| Tecla | Acción |
|-------|--------|
| `j` / `↓` | Bajar en la lista de vaults |
| `k` / `↑` | Subir en la lista de vaults |
| `Enter` | Abrir vault seleccionado |
| `n` | Crear nuevo vault |
| `o` | Abrir vault desde el explorador de archivos |
| `s` | Cambiar directorio por defecto para nuevos vaults |
| `d` | Eliminar vault de la lista de recientes |
| `q` | Salir |

### Modo Normal

| Tecla | Acción |
|-------|--------|
| `e` / `i` | Entrar modo Insert (editar nota) |
| `:` | Entrar modo Command |
| `Tab` | Siguiente tab |
| `Shift+Tab` | Tab anterior |
| `Ctrl+W` | Cerrar tab activo |
| `j` / `↓` | Mover selección hacia abajo en el sidebar |
| `k` / `↑` | Mover selección hacia arriba en el sidebar |
| `h` / `←` | Colapsar carpeta en el sidebar |
| `l` / `→` | Expandir carpeta en el sidebar |
| `Enter` | Abrir nota seleccionada / expandir o colapsar carpeta |
| `g` | Seguir wikilink bajo el cursor (`[[nota]]`) |
| `d` | Scroll preview hacia abajo (en modos Split y Preview) |
| `u` | Scroll preview hacia arriba (en modos Split y Preview) |
| `Ctrl+V` | Ciclar entre vistas: Editor → Split → Preview |
| `Ctrl+T` | Abrir Configuración (Extensiones + Temas) |
| `Ctrl+H` | Ir a pantalla de inicio |
| `Ctrl+S` | Guardar nota activa |
| `r` | Renombrar nota activa (abre Command con `:rename ` pre-escrito) |
| `D` / `Ctrl+D` | Eliminar nota activa (abre Command con `:delete` pre-escrito, Enter confirma) |
| `Ctrl+C` | Copiar contenido completo de la nota al clipboard |
| `Ctrl+Q` | Guardar todo y salir |

### Modo Insert

| Tecla | Acción |
|-------|--------|
| `Esc` | Volver a Normal (guarda automáticamente si hay cambios) |
| `Ctrl+S` | Guardar |
| `Ctrl+Z` | Deshacer |
| `Ctrl+Y` | Rehacer |
| `Ctrl+C` / `Ctrl+Shift+C` | Copiar selección al clipboard del sistema |
| `Ctrl+Backspace` / `Ctrl+W` / `Ctrl+H` | Eliminar palabra anterior |
| `Enter` | Nueva línea (continúa listas automáticamente) |
| `"` / `'` / `` ` `` | Auto-cierra el par de caracteres |

### Modo Command

Se activa con `:` desde Normal. Confirmar con `Enter`, cancelar con `Esc`.

| Comando | Acción |
|---------|--------|
| `:w` | Guardar nota activa |
| `:q` | Cerrar tab (falla si hay cambios sin guardar) |
| `:q!` | Cerrar tab forzando (descarta cambios) |
| `:wq` | Guardar y cerrar tab |
| `:qa` / `:qa!` | Cerrar la aplicación |
| `:new <nombre>` | Crear nueva nota |
| `:mkdir <nombre>` | Crear carpeta |
| `:vault <ruta>` | Cambiar de vault |
| `:home` | Ir a pantalla de inicio |
| `:export-tema <nombre>` | Exportar tema actual a archivo |
| `:import-tema <nombre>` | Importar tema desde archivo |
| `:temas` | Listar temas exportados |
| `:help` | Mostrar lista de comandos |
| `:rename <nombre>` / `:mv <nombre>` | Renombrar la nota activa |
| `:delete` / `:rm` | Eliminar la nota activa (cierra el tab y borra el archivo) |
| `:ext <acción>` | Gestionar extensiones (ver sección Extensiones) |

### Configuración — Settings (`Ctrl+T`)

| Tecla | Acción |
|-------|--------|
| `Tab` | Cambiar entre tabs (Extensiones ↔ Temas) |
| `1` | Ir a tab Extensiones |
| `2` | Ir a tab Temas |
| `Esc` | Cerrar configuración |

**Tab Extensiones:**

| Tecla | Acción |
|-------|--------|
| `j` / `↓` | Bajar en la lista |
| `k` / `↑` | Subir en la lista |
| `Space` / `Enter` | Activar extensión (muestra aviso de seguridad) |
| `Space` / `Enter` | Desactivar extensión (si ya está activa, sin aviso) |
| `Delete` | Desinstalar extensión |

**Tab Temas:**

| Tecla | Acción |
|-------|--------|
| `←` / `h` | Preset anterior |
| `→` / `l` | Preset siguiente |
| `Enter` | Aplicar preset seleccionado |
| `j` / `↓` | Bajar a campos de color |
| `k` / `↑` | Subir a presets |
| `Enter` (en campo) | Editar color en hex |
| `e` | Exportar tema actual con nombre |
| `i` | Importar tema guardado (por nombre) |

---

## Vistas

Cicla con `Ctrl+V`:

| Vista | Descripción |
|-------|-------------|
| **Editor** | Sidebar + editor de texto |
| **Split** | Sidebar + editor + preview en tiempo real |
| **Preview** | Sidebar + preview Markdown renderizado |

En los bloques de código del preview aparece un botón `[copy]` en la cabecera. Puedes hacer clic con el mouse sobre ese botón para copiar el bloque completo al clipboard.

---

## Sidebar

El sidebar izquierdo muestra el árbol de archivos del vault. Soporta carpetas colapsables. La selección activa se resalta. Solo muestra archivos `.md`.

---

## Wikilinks

Escribe `[[nombre-de-nota]]` en cualquier nota. En modo Normal, posiciona el cursor sobre el link y presiona `g` para abrirla en un nuevo tab. Si la nota no existe, Mimic sugiere crearla con `:new`.

---

## Autosave

- Al salir del modo Insert (`Esc`), la nota se guarda automáticamente si tiene cambios.
- Después de 30 segundos de inactividad se guardan todas las notas con cambios pendientes.

---

## Temas

Cuatro presets incluidos: **Default**, **Matrix**, **SDRX**, **Custom**.

El preset Custom permite editar los 11 colores de la interfaz individualmente (fondo, texto, acento, encabezados, links, bordes, sidebar, tabs, cursor). Los colores se ingresan en formato hexadecimal (`#RRGGBB`).

Los temas personalizados se pueden exportar como archivos `.toml` y compartir:

```
:export-tema mi-tema       → ~/.config/sdrx-mimic/themes/mi-tema.toml
:import-tema mi-tema       → importa y aplica
:temas                     → lista los exportados
```

---

## Extensiones [BETA]

Sistema de plugins que permite extender Mimic sin modificar el código fuente. Las extensiones las crea la comunidad.

> **Aviso de seguridad:** Las extensiones son código de terceros no revisado por SDRX Mimic. Instala solo de fuentes confiables. El creador de SDRX Mimic no se responsabiliza de daños causados por extensiones de terceros. Siempre se solicita confirmación antes de activar cualquier extensión.

### Estructura de una extensión

```
mi-extension/
├── manifest.toml
└── main.lua         (o main.rhai)
```

**manifest.toml mínimo:**
```toml
name        = "mi-extension"
version     = "0.1.0"
author      = "Nombre"
description = "Descripción"
language    = "lua"       # o "rhai"
enabled     = false
permissions = ["commands", "hooks.save"]
```

### Lenguajes soportados

- **Lua 5.4** — sandboxed (sin `io`, `os`, `package`, `debug` por defecto)
- **Rhai** — sandboxed (límite de operaciones, sin acceso a filesystem por defecto)

### Permisos disponibles

| Permiso | Descripción |
|---------|-------------|
| `commands` | Registrar comandos custom (`:mi-comando`) |
| `hooks.save` | Hook al guardar una nota |
| `hooks.open` | Hook al abrir una nota |
| `hooks.mode` | Hook al cambiar de modo |
| `markdown` | Registrar renderers para bloques de código |
| `fs.write` ⚠ | Escribir archivos en el vault |
| `process.run` ⚠ | Ejecutar subprocesos del sistema |

### Gestión desde terminal

```bash
mimic ext list                      # listar extensiones instaladas
mimic ext install /ruta/extension   # instalar desde carpeta local
mimic ext enable <nombre>           # activar
mimic ext disable <nombre>          # desactivar
mimic ext remove <nombre>           # desinstalar
```

### Gestión desde el TUI

**`Ctrl+T`** → tab **Extensiones [BETA]**

### Comandos in-app

```
:ext list
:ext install <ruta>
:ext enable <nombre>
:ext disable <nombre>
:ext remove <nombre>
```

### Extensión de ejemplo incluida

```bash
mimic ext install ./ejemplos-extensiones/hola-mundo
```

Muestra mensajes en la barra de estado al abrir/guardar notas y registra el comando `:hola`.

### Documentación para crear extensiones

Ver [EXTENSIONS.md](EXTENSIONS.md) — guía completa con API, ejemplos en Lua y Rhai, y referencia de permisos.

---

## Configuración

Todo en `~/.config/sdrx-mimic/`:

```
~/.config/sdrx-mimic/
├── config.toml          ← preferencias, vaults recientes, tema activo, vista por defecto
├── themes/              ← temas exportados (.toml)
└── extensions/          ← extensiones instaladas (una carpeta por extensión)
```

Mimic recuerda hasta 20 vaults recientes y el último directorio por defecto para nuevos vaults.

---

## Clipboard

Usa `wl-copy` (Wayland), `xclip` o `xsel` (X11) según lo disponible. Si ninguno está instalado o no hay servidor gráfico disponible (ej. sesión SSH sin forwarding), el clipboard no funciona pero la app no muestra errores.

---

## Stack técnico

| Componente | Librería |
|------------|---------|
| TUI framework | `ratatui` 0.29 |
| Terminal backend | `crossterm` 0.28 |
| Editor de texto | `tui-textarea` 0.7 |
| Parser Markdown | `pulldown-cmark` 0.12 |
| File tree | `walkdir` 2 |
| CLI | `clap` 4 |
| Serialización | `serde` + `toml` 0.8 |
| Rutas de config | `dirs` 6 |
| Wikilinks | `regex` 1 |
| Runtime Lua | `mlua` 0.10 (Lua 5.4 embebido) |
| Runtime Rhai | `rhai` 1 |

---

## Roadmap

- [ ] Tienda de extensiones
- [ ] Renderizado de imágenes en terminal (Kitty/Sixel)
- [ ] Exportar notas a HTML / PDF
- [ ] Sincronización de vaults
- [ ] Visualización de grafo de notas
- [ ] Ejecución de código en bloques de código (Python, JavaScript)

---

## Licencia

Por definir.
