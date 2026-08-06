player_controller = player_controller or {}
player_controller.__index = player_controller

function player_controller.new(node)
    return setmetatable({ node = node }, player_controller)
end

local GRAVITY = 0.2
local MAX_FALL = 8.0
local RUN_SPEED = 0.2
local JUMP_VELOCITY = -1.1

function player_controller:on_tick()
    local me = self.node
    local input = engine.get_node("__input")
    local dir = input.direction
    local vel = me.velocity

    local grounded = vel[2] == 0.0

    vel[2] = vel[2] + GRAVITY
    if vel[2] > MAX_FALL then
        vel[2] = MAX_FALL
    end

    vel[1] = dir[1] * RUN_SPEED

    if input.action and grounded then
        vel[2] = JUMP_VELOCITY
    end

    me.grounded = grounded
    me.velocity = vel
end

function player_controller:on_signal(signal_name, args)
    if signal_name == "collide" then
        local me = self.node
        if args.a == "goal" or args.b == "goal" then
            me.won = true
        end
    end
end
