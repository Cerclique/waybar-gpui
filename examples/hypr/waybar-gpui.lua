-- Optional: add to ~/.config/hypr/hyprland.lua (Hyprland >= 0.55 Lua config).
-- Every waybar-gpui program's popover is a layer surface with namespace
-- "waybar-gpui:<id>"; check with `hyprctl layers`. It needs no rules to float
-- or position itself. Match "^waybar-gpui:<id>$" for one program only.
-- The surface covers the whole output and is transparent outside the panel
-- card: ignore_alpha keeps the blur on the card only.
hl.layer_rule({
  match = { namespace = "^waybar-gpui:" },
  blur = true,
  ignore_alpha = 0.3,
  animation = "fade",
})
