-- Extensión de prueba: hola-mundo
-- Muestra un mensaje al abrir o guardar cualquier nota

function on_open(path, content)
    local nombre = path:match("[^/]+$") or path
    mimic.notify("✓ Extensión funcionando — abriste: " .. nombre)
end

function on_save(path, content)
    local nombre = path:match("[^/]+$") or path
    local palabras = 0
    for _ in content:gmatch("%S+") do
        palabras = palabras + 1
    end
    mimic.notify("✓ Guardado: " .. nombre .. " (" .. palabras .. " palabras)")
end

-- Comando :hola para verificar que la extensión responde
mimic.register_command("hola", function(args)
    return "¡La extensión hola-mundo está funcionando!"
end)
