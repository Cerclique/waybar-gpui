-- Optional: add to ~/.config/hypr/hyprland.lua (Hyprland >= 0.55 Lua config).
-- The popover is a layer surface with namespace "waybar-popup"; check with
-- `hyprctl layers`. It needs no window rules to float or position itself.
-- The surface covers the whole output and is transparent outside the panel
-- card: ignore_alpha keeps the blur on the card only.
hl.layer_rule({
  match = { namespace = "^waybar-popup$" },
  blur = true,
  ignore_alpha = 0.3,
  animation = "fade",
})
