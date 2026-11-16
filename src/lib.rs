//! gtaol-doctor: diagnostics for GTA V Enhanced + BattlEye on Linux.
//!
//! Everything here is read-only except `hosts::apply` and
//! `game::quarantine_startup_meta`, which are explicit, backed up and reversible.
pub mod beruntime;
pub mod ctx;
pub mod diagnose;
pub mod game;
pub mod hosts;
pub mod launchers;
pub mod report;
pub mod system;
pub mod titles;
