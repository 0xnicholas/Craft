Enemy = Enemy or {}

function Enemy:on_tick()
    local player = engine.find_node("player")
    if not player then return end

    local px, py = player.position[1], player.position[2]
    local ex, ey = self.node.position[1], self.node.position[2]
    local dx = px - ex
    local dy = py - ey

    if math.abs(dx) < 0.6 and math.abs(dy) < 0.6 then
        self.node.velocity = {0, 0}
        return
    end

    if math.abs(dx) > math.abs(dy) then
        self.node.velocity = {dx > 0 and 1 or -1, 0}
    else
        self.node.velocity = {0, dy > 0 and 1 or -1}
    end
end
