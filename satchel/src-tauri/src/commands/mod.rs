//! Tauri commands: the only surface the frontend can call. (The window's capability only grants
//! `core:default`, so the frontend can't touch the filesystem, shell, or dialogs directly.)

pub mod settings;
pub mod system;
