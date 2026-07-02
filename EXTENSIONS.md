# SDRX Mimic — Guía de Extensiones [BETA]

> **Aviso de seguridad:** Las extensiones son código de terceros no revisado por los creadores de SDRX Mimic. Instala únicamente extensiones de fuentes en las que confíes. El creador de SDRX Mimic no se responsabiliza de daños causados por extensiones de terceros.

---

## ¿Qué es una extensión?

Una extensión (o plugin) te permite añadir nuevas funcionalidades a SDRX Mimic sin modificar el código fuente. Puedes crear extensiones en **Lua** o **Rhai**.

---

## Estructura de una extensión

Cada extensión es una **carpeta** con al menos dos archivos:

```
mi-extension/
├── manifest.toml    ← metadata obligatorio
└── main.lua         ← código de la extensión (o main.rhai)
```

### manifest.toml

```toml
name        = "mi-extension"
version     = "0.1.0"
author      = "Tu Nombre"
description = "Descripción breve de lo que hace la extensión"
language    = "lua"          # o "rhai"
enabled     = false          # siempre false al instalar, el usuario activa manualmente

# Permisos que solicita la extensión (solo pide los que realmente necesitas)
permissions = ["commands", "hooks.save"]
```

### Permisos disponibles

| Permiso         | Descripción                                                          | Peligroso |
|-----------------|----------------------------------------------------------------------|-----------|
| `commands`      | Registrar comandos custom (`:mi-comando`)                           | No        |
| `hooks.save`    | Ejecutar código cuando el usuario guarda una nota                   | No        |
| `hooks.open`    | Ejecutar código cuando el usuario abre una nota                     | No        |
| `hooks.mode`    | Ejecutar código cuando cambia el modo de la app                     | No        |
| `markdown`      | Registrar renderers custom para bloques de código markdown          | No        |
| `fs.write`      | Escribir archivos en el vault (acceso a `mimic_write_file`)         | ⚠ Sí     |
| `process.run`   | Ejecutar subprocesos del sistema (acceso a `mimic_run`)             | ⚠ Sí     |

> **Nota:** Los permisos peligrosos (`fs.write`, `process.run`) mostrarán una advertencia extra al usuario antes de activar la extensión.

---

## API en Lua

El objeto global `mimic` expone las siguientes funciones:

### `mimic.notify(mensaje)`
Muestra un mensaje en la barra de estado de Mimic.
```lua
mimic.notify("¡Hola desde mi extensión!")
```

### `mimic.register_command(nombre, función)`
Registra un comando disponible en el modo Command (`:nombre`).
```lua
mimic.register_command("hola", function(args)
    return "¡Hola, " .. (args[1] or "mundo") .. "!"
end)
-- Uso en Mimic: :hola Mundo
```

### `mimic.on(evento, función)`
Registra un hook para un evento del sistema.
```lua
mimic.on("on_save", function(path, content)
    mimic.notify("Nota guardada: " .. path)
end)
```

Eventos disponibles: `on_save`, `on_open`, `on_mode_change`, `on_markdown_block`

### Funciones opcionales de nivel superior (alternativa a `mimic.on`)

Puedes definir funciones con nombres de convención directamente:

```lua
-- Llamada al guardar una nota
function on_save(path, content)
    mimic.notify("Guardado: " .. path)
end

-- Llamada al abrir una nota
function on_open(path, content)
    -- código aquí
end

-- Llamada al cambiar de modo (Normal → Insert, etc.)
function on_mode_change(from_mode, to_mode)
    -- código aquí
end

-- Llamada para bloques de código en markdown
-- Retorna el texto renderizado, o nil para comportamiento default
function on_markdown_block(lang, code)
    if lang == "mermaid" then
        return "[diagrama mermaid: " .. #code .. " bytes]"
    end
    return nil  -- usa el renderer por defecto
end
```

### Funciones de comandos (alternativa a `mimic.register_command`)

Puedes definir una tabla `commands` en el nivel superior:

```lua
commands = {
    ["hola"] = function(args)
        return "¡Hola, " .. (args[1] or "mundo") .. "!"
    end,
    ["version"] = function(args)
        return "Mi extensión v0.1.0"
    end,
}
```

### Permisos peligrosos (solo si declarados en manifest.toml)

```lua
-- Requiere permiso "fs.write"
-- Escribe un archivo en el vault
local ok = mimic_write_file("/ruta/al/vault/nota.md", "contenido")

-- Requiere permiso "process.run"
-- Ejecuta un comando del sistema y retorna stdout
local output = mimic_run("python3", {"-c", "print('hola')"})
```

---

## API en Rhai

En Rhai defines funciones de nivel superior con los mismos nombres de convención:

```rhai
// Llamada al guardar
fn on_save(path, content) {
    mimic_notify("Guardado: " + path);
}

// Llamada al abrir
fn on_open(path, content) {
    // código aquí
}

// Comandos: se llama run_command(nombre, args_array)
fn run_command(name, args) {
    if name == "hola" {
        return "¡Hola desde Rhai!";
    }
    return ();  // nil — no manejado
}

// Renderer de bloques markdown
fn on_markdown_block(lang, code) {
    if lang == "rhai-demo" {
        return "[rhai: " + code.len().to_string() + " chars]";
    }
    return ();
}
```

### Funciones de API disponibles en Rhai

```rhai
mimic_notify("mensaje");           // muestra mensaje en status bar

// Solo si fs.write declarado en permissions:
let ok = mimic_write_file("/ruta", "contenido");

// Solo si process.run declarado en permissions:
let output = mimic_run("python3", ["-c", "print('ok')"]);
```

---

## Instalar una extensión

### Desde el TUI (modo Command)
```
:ext install /ruta/a/mi-extension
```

### Desde la terminal
```bash
mimic ext install /ruta/a/mi-extension
```

### Manualmente (copiar la carpeta)
```bash
cp -r mi-extension ~/.config/sdrx-mimic/extensions/
```

La extensión se instala **desactivada por defecto**. Para activarla:

- Abre la configuración con `Ctrl+T` → tab "Extensiones [BETA]"
- Selecciona la extensión con `j/k`
- Presiona `Space` o `Enter`
- Confirma la advertencia de seguridad con `Y`

O desde la terminal:
```bash
mimic ext enable mi-extension
```

---

## Comandos de gestión

### Terminal
```bash
mimic ext list                      # listar extensiones instaladas
mimic ext install /ruta/ext         # instalar desde carpeta
mimic ext remove nombre             # desinstalar
mimic ext enable nombre             # activar
mimic ext disable nombre            # desactivar
```

### Dentro del TUI (modo Command)
```
:ext list
:ext install /ruta/ext
:ext remove nombre
:ext enable nombre
:ext disable nombre
```

---

## Ejemplo completo: extensión "word-count"

**Estructura:**
```
word-count/
├── manifest.toml
└── main.lua
```

**manifest.toml:**
```toml
name        = "word-count"
version     = "0.1.0"
author      = "Tu Nombre"
description = "Cuenta palabras en la nota actual al guardar"
language    = "lua"
enabled     = false
permissions = ["hooks.save", "commands"]
```

**main.lua:**
```lua
-- Contar palabras al guardar
function on_save(path, content)
    local count = 0
    for _ in content:gmatch("%S+") do
        count = count + 1
    end
    mimic.notify(string.format("Guardado: %d palabras en %s", count, path:match("[^/]+$")))
end

-- Comando :wc para ver conteo en cualquier momento
mimic.register_command("wc", function(args)
    return "Usa :w para ver el conteo al guardar"
end)
```

---

## Ejemplo completo: extensión en Rhai

**manifest.toml:**
```toml
name        = "saludo-rhai"
version     = "0.1.0"
author      = "Tu Nombre"
description = "Extensión de demostración en Rhai"
language    = "rhai"
enabled     = false
permissions = ["commands"]
```

**main.rhai:**
```rhai
fn run_command(name, args) {
    if name == "saludo" {
        let quien = if args.len() > 0 { args[0] } else { "mundo" };
        return "¡Hola, " + quien + "! (desde Rhai)";
    }
    return ();
}
```

---

## Sandbox y seguridad

Las extensiones se ejecutan en un entorno **restringido**:

**Lua:**
- Solo tiene acceso a las librerías: `math`, `string`, `table`, `utf8`
- `io`, `os`, `package`, `debug` están **bloqueados** por defecto
- Solo se habilitan si el manifest declara los permisos correspondientes

**Rhai:**
- Límite de 100,000 operaciones por llamada (protección contra loops infinitos)
- Límite de 32 niveles de recursión
- Sin acceso a filesystem ni procesos por defecto
- `eval` deshabilitado

**En ambos casos:**
- Las extensiones no pueden modificar el código fuente de SDRX Mimic
- Las extensiones solo pueden AÑADIR funcionalidad, no reemplazar comportamiento core
- El usuario siempre confirma antes de activar cualquier extensión

---

## Distribución

Para distribuir tu extensión:
1. Empaqueta la carpeta como un `.zip` o `.tar.gz`
2. Los usuarios la descomprimen y usan `mimic ext install <carpeta>`
3. En el futuro habrá una tienda oficial de extensiones

---

## Estado: BETA

El sistema de extensiones está en **fase beta**. La API puede cambiar en versiones futuras. Si encuentras bugs o tienes sugerencias, repórtalos en el repositorio del proyecto.
