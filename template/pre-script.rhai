// The program ID names the program's socket, log file and Hyprland layer
// namespace: same rule as waybar_gpui's App::new.
if !variable::is_set("id") {
    let id = variable::get("project-name");
    try {
        id = variable::prompt("Program ID (a-z, 0-9, '-'; keep it stable)", id, "^[a-z0-9][a-z0-9-]*$");
    } catch {
        // No terminal (`--silent`, CI): the project name, if it's a valid ID.
        let valid = id.len() > 0 && !id.starts_with("-");
        for c in id.chars() {
            if !"abcdefghijklmnopqrstuvwxyz0123456789-".contains(c) {
                valid = false;
            }
        }
        if !valid {
            throw `project name "${id}" isn't a valid program ID; pass --define id=<id>`;
        }
    }
    variable::set("id", id);
}
