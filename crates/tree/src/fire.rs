//! Mutable tree data structure.
//!
//! See [`ForestFire`].

use crate::{
    ashes::{Ashes, BranchId as AshBranchId, Node as AshNode},
    internal::BranchIdImpl,
};

define_branch_id!(
    /// The ID for some branch of a [`ForestFire`].
    ///
    /// Branch IDs should generally only be used within the tree where they
    /// were obtained, but, technically speaking, there is nothing barring
    /// you from doing it anyway.
    struct BranchId
);

struct Node<T> {
    parent: BranchId,
    payload: T,
}

/// Mutable tree data structure.
///
/// A `ForestFire` represents an in-progress tree data structure; it cannot
/// be traversed like a normal tree (the children of a given node cannot
/// be retrieved), but it can eventually be [burned] and thus converted to
/// the more friendly [`Ashes`].
///
/// Each node of a `ForestFire` has a single instance of `T`, called the
/// "payload". The one exception is the root node, which can never have any
/// payloads.
///
/// [burned]: Self::burn
pub struct ForestFire<T> {
    nodes: Vec<Node<T>>,
}

const _: () = {
    assert!(
        size_of::<Node<()>>() != 0,
        "Node<_> should not be zero sized"
    );
};

#[cold]
fn root_panic() -> ! {
    panic!("given ID must not be {root}", root = BranchIdImpl::ROOT_STR)
}

impl<T> ForestFire<T> {
    /// Constructs a new, empty `ForestFire<T>`.
    pub const fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Returns the number of nodes in this tree.
    ///
    /// This does not include the root node.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Checks whether there is a branch with the given branch ID.
    ///
    /// Branch IDs given out by a `ForestFire` are valid for the entirety of that `ForestFire`'s lifetime,
    /// so there is never a need to check if you're sure your branch ID came from this exact `ForestFire`
    /// instance.
    ///
    /// Always returns `true` for [`BranchId::ROOT`].
    pub fn exists(&self, branch: BranchId) -> bool {
        if branch.is_root() {
            return true;
        }

        self.nodes.get(branch.value()).is_some()
    }

    /// Returns the parent ID of a given branch, or `None` if it is [`BranchId::ROOT`].
    ///
    /// # Panics
    ///
    /// Panics if `of` is not an [existing](Self::exists) branch.
    pub fn parent(&self, of: BranchId) -> Option<BranchId> {
        if of.is_root() {
            None
        } else {
            match self.nodes.get(of.value()) {
                Some(node) => Some(node.parent),
                None => of.indexing_panic(),
            }
        }
    }

    /// Returns a shared reference to the payload of a given branch, or `None` if it is [`BranchId::ROOT`].
    ///
    /// A `&mut` reference may be obtained from [`get_payload_mut`](#method.get_payload_mut).
    ///
    /// # Panics
    ///
    /// Panics if `of` is not an [existing](Self::exists) branch.
    pub fn get_payload(&self, of: BranchId) -> Option<&T> {
        if of.is_root() {
            None
        } else {
            Some(
                &self
                    .nodes
                    .get(of.value())
                    .unwrap_or_else(|| of.indexing_panic())
                    .payload,
            )
        }
    }

    /// Returns a mutable (exclusive) refernece to the payload of a given branch, or `None` if it is
    /// [`BranchId::ROOT`].
    ///
    /// A shared reference may be obtained from [`get_payload`](#method.get_payload).
    ///
    /// # Panics
    ///
    /// Panics if `of` is not an [existing](Self::exists) branch.
    pub fn get_payload_mut(&mut self, of: BranchId) -> Option<&mut T> {
        if of.is_root() {
            None
        } else {
            Some(
                &mut self
                    .nodes
                    .get_mut(of.value())
                    .unwrap_or_else(|| of.indexing_panic())
                    .payload,
            )
        }
    }

    /// Returns a shared reference to the payload of a given branch.
    ///
    /// # Panics
    ///
    /// Panics if `of` is not an [existing](Self::exists) branch, or if it is [`BranchId::ROOT`]. For a
    /// non-panicking variant, use [`get_payload`](#method.get_payload).
    pub fn payload(&self, of: BranchId) -> &T {
        self.get_payload(of).unwrap_or_else(|| root_panic())
    }

    /// Returns a mutable (exclusive) refernece to the payload of a given branch.
    ///
    /// # Panics
    ///
    /// Panics if `of` is not an [existing](Self::exists) branch, or if it is [`BranchId::ROOT`]. For a
    /// non-panicking variant, use [`get_payload_mut`](#method.get_payload_mut).
    pub fn payload_mut(&mut self, of: BranchId) -> &mut T {
        self.get_payload_mut(of).unwrap_or_else(|| root_panic())
    }

    /// Appends a new child to the provided parent, with the provided payload.
    ///
    /// `parent` may be any branch ID previously given by this `ForestFire`, or [`BranchId::ROOT`].
    ///
    /// # Panics
    ///
    /// Panics on any of:
    ///  - `of` is not an [existing](Self::exists) branch
    ///  - Capacity of the internal node buffer overflows `isize::MAX` bytes.
    ///  - Memory runs out.
    pub fn branch(&mut self, parent: BranchId, payload: T) -> BranchId {
        // this assertion is not strictly required: the only effect of providing
        // a valid parent ID is that once the forest is burned, it will panic due
        // to out-of-bounds access. but i think its better to fail here than later
        // on
        if !self.exists(parent) {
            parent.indexing_panic()
        }

        let id = self.nodes.len();

        // since allocations are not allowed to exceed isize::MAX *bytes*, a Vec
        // will most definitely not reach usize::MAX *instances* as long as the
        // contained value is non-ZST
        debug_assert_ne!(id, usize::MAX);
        // Node should not be a ZST in any normal situation
        //  (technically, if Payload is a zero-variant struct, then Node will
        //   likely be zero-sized. so we must not create a compile-time
        //   check as this function could somehow be monomorphized for a
        //   Payload of `Infallible` or `!`, but a runtime check is fine
        //   because a runtime check will never be reached soundly with
        //   an alive instance of a zero-variant struct)
        debug_assert_ne!(size_of::<Node<T>>(), 0);

        self.nodes.push(Node { parent, payload });

        BranchId::new_branch(id)
    }

    /// Returns the branch ID which would be returned by the next call to [`branch`].
    ///
    /// This will never be [BranchId::ROOT].
    ///
    /// [`branch`]: Self::branch
    pub fn next_id(&self) -> BranchId {
        BranchId::new_branch(self.nodes.len())
    }

    /// Finishes building this tree and creates an instance of [Ashes].
    ///
    /// # Performance considerations
    ///
    /// This method will perform multiple allocations and will iterate over the existing nodes
    /// multiple times; it is likely to take a fairly large amount of time.
    ///
    /// `ForestFire` is meant for places where the tree is often discarded (mainly: capturing traces
    /// of test functions. the tree is only required when the test fails); if you always need to
    /// create a traversable tree _and_ you need to do it fast, then `ForestFire` is likely not the
    /// right choice for you.
    ///
    /// # Panics
    ///
    /// Panics if memory runs out or if any of the internal buffers overflow `isize::MAX` bytes.
    pub fn burn(self) -> Ashes<T> {
        // todo: this could do with a lot of optimizing

        // let mut new2old: Vec<usize> = (0..self.nodes.len()).collect();
        // new2old.sort_by_key(|&x| self.nodes[x].parent);

        let mut nodes: Vec<AshNode<T>> = self
            .nodes
            .into_iter()
            .enumerate()
            .map(|(i, Node { parent, payload })| AshNode {
                // parent will use old-style indexing for now
                parent: AshBranchId::new(parent.value()),
                payload,
                children: 0..0,
                old_idx: i,
            })
            .collect();

        nodes.sort_by_key(|x| x.parent);
        let mut old2new = (0..nodes.len()).collect::<Vec<_>>();
        old2new.sort_unstable_by_key(|&idx| nodes[idx].old_idx);

        for node in &mut nodes {
            let parent = if node.parent.is_root() {
                AshBranchId::ROOT
            } else {
                AshBranchId::new_branch(old2new[node.parent.value()])
            };
            node.parent = parent;
        }

        let mut last_parent = AshBranchId::ROOT;
        let mut child_lo = 0;
        let mut root_children = usize::MAX..usize::MAX;

        macro_rules! flush_parent {
            ($end:expr) => {{
                let end: usize = $end;
                if last_parent.is_root() {
                    root_children = child_lo..end;
                } else {
                    nodes[last_parent.value()].children = child_lo..end;
                }
            }};
        }

        for i in 0..nodes.len() {
            let parent = nodes[i].parent;
            if last_parent != parent {
                // child_lo will be 0 on the first seen node (which will also have an idx of 0)
                if child_lo != i {
                    flush_parent!(i)
                }

                last_parent = parent;
                child_lo = i;
            }
        }
        // if nodes is empty, then this will simply set root_children to 0..0
        // since last_parent will be ROOT
        flush_parent!(nodes.len());

        Ashes {
            nodes,
            root_children,
        }
    }
}

impl<T> Default for ForestFire<T> {
    fn default() -> Self {
        Self::new()
    }
}
