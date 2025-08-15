<!-- this is copied from the crate-level docs -->

A tree data structure which can be built quickly and then burned to an immutable version in one go.

- `ForestFire` is the mutable version of the tree:
    - Adding a new node (`ForestFire::branch`) basically amounts to a `Vec::push`.
    - Can be "burned" into the immutable `Ashes` where it can be traversed as a tree.
    - Nodes can be added to any part of the tree at any time.
    - Each node contains a generic payload which you specify.
- `Ashes` is the immutable version of the tree:
    - While the tree structure is immutable, the payloads are fully available mutably.
    - Children maintain insertion order.
    - Can be de/serialized.

This crate is part of the [`forrust_fire`](https://github.com/purple-ic/forrust_fire) collection.