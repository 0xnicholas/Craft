--- @meta

--- @class Engine
--- @field get_node fun(id: string): Node | nil
--- @field emit fun(signal: string, args: table)
--- @field call_system fun(name: string, args: table): any
local Engine = {}

--- @field scene Scene
Engine.scene = nil

--- @class Scene
--- @field tick fun()
local Scene = {}

--- @class Node
--- @field id string
--- @field position Vec2
--- @field [string] any
local Node = {}

--- @class Vec2
--- @field [1] number
--- @field [2] number
local Vec2 = {}

--- @class SignalBus
--- @field emit fun(name: string, args: table)
local SignalBus = {}

return {}