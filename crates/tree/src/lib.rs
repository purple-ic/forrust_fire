//! A tree data structure which can be built quickly and then burnt to an immutable version in one go.
//!
//! - [`ForestFire`] is the mutable version of the tree:
//!     - Adding a new node ([`ForestFire::branch`]) basically amounts to a [`Vec::push`].
//!     - Can be ["burned"](fire::ForestFire::burn) into the immutable [`Ashes`] where it can be traversed as a tree.
//!     - Nodes can be added to any part of the tree at any time.
//!     - Each node contains a generic payload which you specify.
//! - [`Ashes`] is the immutable version of the tree:
//!     - While the tree structure is immutable, the payloads are fully available mutably.
//!     - Children maintain insertion order.
//!     - Can be [de/serialized](ashes::serde).
//!
//! [`ForestFire`]: fire::ForestFire
//! [`ForestFire::branch`]: fire::ForestFire::branch
//! [`Ashes`]: ashes::Ashes

#![warn(missing_docs)]

#[macro_use]
pub(crate) mod internal;

pub mod ashes;
pub mod fire;

#[cfg(test)]
mod test;
