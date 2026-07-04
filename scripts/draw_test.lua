-- teste_desenho.lua

function on_init()
    log_code("=== Teste da API de Desenho Iniciado ===")
end

function on_frame()
    if inpt then
        local mx = inpt.xmouse
        local my = inpt.ymouse

        local cor = "#ffffff"
        if inpt.leftclick then
            cor = "#00ff00"
        end

        draw_box(mx - 8, my - 8, mx + 8, my + 8, cor)

        draw_text(mx + 12, my - 4, string.format("X: %d, Y: %d", mx, my))
    end
end