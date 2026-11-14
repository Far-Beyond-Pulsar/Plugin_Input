# Input examples

Read-only examples of the two file types the Input plugin registers. Copy one
into your project's assets to start from it.

## Actions (`actions/*.inputaction`)

An action is something the player can do. Every file here is plain JSON; the
`description` field says what the example shows.

| File | Shows |
|---|---|
| `jump` | a digital action with a Pressed trigger |
| `move` | a 2D action with a radial dead zone |
| `look` | a 2D action with a response curve and Smooth |
| `sprint` | Down trigger, `consume_input` off |
| `charged_shot` | Hold And Release |
| `dash` | Double Tap |
| `throttle` | a 1D action, Smooth and Clamp |
| `rapid_fire` | Pulse |
| `fly` | a 3D action with a Scale modifier |
| `crouch` | Tap and Hold on one action |
| `modifier_ctrl`, `quick_save` | a Chorded trigger (Ctrl + key) |
| `menu_navigate`, `menu_confirm` | actions that keep firing while paused |
| `aim` | a lowered actuation threshold |
| `throw` | a Released trigger |

## Mappings (`mappings/*.inputmap`)

A mapping binds keys to actions. It refers to an action by its `id` (the
`hint` is only where the file was). The ids in these examples are fixed, so
the mappings find the example actions as long as both are in the project. Each
entry's `note` explains it.

| File | Shows |
|---|---|
| `gameplay` | on-foot controls: WASD built from swizzle and negate, sticks, mouse look, combat |
| `menu` | a high-priority (100) mapping that hides the gameplay keys while a menu is open |
| `vehicle` | an analogue throttle, a mid-priority layer, a disabled entry |
| `free_camera` | a 3D action with Space and Ctrl on Z |

When you copy an action or mapping to make your own, give it a new `id`
(delete the field and the editor stamps a fresh one the first time it opens
the file), or two files will claim the same identity.
