/// This module contains data about postgres lock modes and their capabilities.
pub mod lock_modes;
/// Locks targeting database objects like tables or indexes, together with their lock modes.
pub mod locks;
/// Postgres object types like tables, indexes, sequences, etc.
pub mod relkinds;
