-- teste_desenho.lua

function on_init()
    log_code("=== Teste da API de Desenho Iniciado ===")
end

function on_frame()
    -- Garante que a tabela de inputs já foi populada pelo Rust
    if inpt then
        local mx = inpt.xmouse
        local my = inpt.ymouse

        -- Define a cor: Branco por padrão, Verde se estiver clicando
        local cor = "#ffffff"
        if inpt.leftclick then
            cor = "#00ff00"
        end

        -- 1. Desenha uma caixa vazia de 16x16 pixels ao redor do ponteiro
        draw_box(mx - 8, my - 8, mx + 8, my + 8, cor)

        -- 2. Desenha o texto com as coordenadas atuais logo ao lado
        draw_text(mx + 12, my - 4, string.format("X: %d, Y: %d", mx, my))
    end
end