# Input plugin

Input **actions** (`.inputaction`), input **mappings** (`.inputmap`), an editor
for each, and the runtime that turns keyboards, mice and controllers into
Gamma events scripts and plugins can hear.

## Assets

**Action**: something the player can do. It says what it reports (on/off, a
number, a 2D or 3D direction), how the value is reshaped (modifiers: dead zone,
scale, negate, swizzle, smooth, response curve, clamp) and when it fires
(triggers: down, pressed, released, hold, hold and release, tap, pulse, double
tap, chorded with another action).

**Mapping**: binds keys to actions. Each entry binds one key and one action;
entries are sorted into categories for the person editing. An entry can carry
its own triggers and modifiers. A mapping has a priority; a higher one sees
input first, and actions that consume input hide their keys from lower ones.

Both files carry a stable `id`. A mapping refers to an action by that id (with
the path it was at as a hint), so renaming or moving an action breaks nothing.

## Editors

* Action editor: settings and the events scripts will hear, triggers and
  modifiers, a timeline that simulates the action against a tap, a hold, a
  double tap or a slow push, the mappings that use it, and problems.
* Mapping editor: the bindings by category (drag to reorder or move, click a
  key or action to change it, "Listen for input" to bind by pressing),
  the project's actions (drag, double-click, or bind with a ready-made set such
  as WASD), details for the selected binding, and problems with one-click fixes.

## Runtime

`PlayerInputComponent` puts a mapping to work on an object. Each frame the
runtime reads keys, mouse buttons, motion and wheel from the event hub, and
controllers (on a thread of their own), evaluates the active mappings, and
publishes the actions in the `input` namespace:

* `input::Jump.started`, `.ongoing`, `.triggered`, `.completed`, `.canceled`
  with `entity`, `value`, `x`, `y`, `z`, `elapsed`, `triggered_for`; on the
  object's channel when the mapping is on an object, otherwise global.
* `input::ActionStarted` ... `ActionCanceled` for everything at once.
* Commands: `input::AddMapping`, `RemoveMapping`, `SetMappingEnabled`.

Edits to a mapping or action on disk take effect in the running game.

## Layout

Everything but `editor` is GPUI-free: `keys`, `model`, `library`, `runtime`,
`preview`.
