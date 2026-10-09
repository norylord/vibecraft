// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
  if !diogen_lib::pty_host() {
    diogen_lib::run();
  }
}
