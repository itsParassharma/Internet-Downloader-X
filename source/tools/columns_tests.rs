// Dependency-free tests of column behavior and compilation of the real Win32
// dialog handlers/resources. This harness does not open a window or prove UI parity.
#![allow(dead_code)]
#[path = "../src/columns.rs"]
mod columns;
#[cfg(windows)]
#[path = "../src/ui/native_columns.rs"]
mod native_columns;
#[cfg(windows)]
#[path = "../src/ui/native_resources.rs"]
mod native_resources;
