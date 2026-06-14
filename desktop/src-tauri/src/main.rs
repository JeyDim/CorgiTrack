// Не открывать лишнее консольное окно на Windows в релизе.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    corgitrack_desktop_lib::run()
}
