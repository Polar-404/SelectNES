-- teste_mouse.lua

local clicked = false

function on_init()
    print("=== testing mouse click ===")
end

function on_frame()
    if inpt then
        
        if inpt.leftclick and not clicked then
            log_code(string.format("Clique detectado via rusta! X: %d, Y: %d", inpt.xmouse, inpt.ymouse))
        end

        clicked = inpt.leftclick
    end
end