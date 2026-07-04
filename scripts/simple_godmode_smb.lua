-- smb_godmode.lua

function on_init()
    print("=== SMB Godmode ===")
end

function on_frame()
    -- gives invencibility
    write_mem_silent(0x079E, 0x05)

    -- maximum lifes
    write_mem_silent(0x075A, 0x63)

    -- forces fire mario
    local estado_atual = read_mem(0x0754)
    if estado_atual < 2 then
        write_mem_silent(0x0754, 2)
    end
end