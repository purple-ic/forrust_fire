//! Immutable tree data structure.
//!
//! See [Ashes].

use std::{
    fmt::{self, Debug, Display},
    ops::Range,
};

#[cfg(feature = "serde")]
pub mod serde;

define_branch_id!(
    /// The ID for some branch of a [`Ashes`].
    ///
    /// Branch IDs should generally only be used within the tree where they
    /// were obtained, but, technically speaking, there is nothing barring
    /// you from doing it anyway.
    struct BranchId
);

#[derive(Clone, Copy, Debug)]
struct RootInfo<'a> {
    children: &'a Range<usize>,
}

/// Shared reference to a branch of [Ashes].
#[derive(Debug)]
pub struct BranchRef<'a, T> {
    // None for <root>
    node: Result<&'a Node<T>, RootInfo<'a>>,
}

impl<'a, T> Clone for BranchRef<'a, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T> Copy for BranchRef<'a, T> {}

impl<'a, T> BranchRef<'a, T> {
    /// Returns whether this is the root branch.
    pub fn is_root(self) -> bool {
        self.node.is_err()
    }

    /// Returns the parent of this branch, or `None` if it is root.
    pub fn parent(self) -> Option<BranchId> {
        match self.node {
            Ok(node) => Some(node.parent),
            Err(_) => None,
        }
    }

    /// Returns the payload of this branch, or `None` if it is root.
    pub fn payload(self) -> Option<&'a T> {
        match self.node {
            Ok(node) => Some(&node.payload),
            Err(_) => None,
        }
    }

    /// Returns an iterator of child IDs for this branch.
    pub fn child_iter(self) -> impl Iterator<Item = BranchId> {
        let range = Range::clone(match self.node {
            Ok(v) => &v.children,
            Err(r) => r.children,
        });
        range.map(BranchId::new_branch)
    }

    /// Returns the range of IDs which are all children of this node.
    pub fn children(self) -> Range<BranchId> {
        let range = match self.node {
            Ok(node) => &node.children,
            Err(root) => root.children,
        };
        child_range(range)
    }

    /// Returns how many children this node has.
    pub fn n_children(self) -> usize {
        children_len(self.children())
    }

    /// Returns the `n`th child's ID.
    pub fn child(self, n: usize) -> BranchId {
        nth_child(self.children(), n)
    }
}

fn children_len(r: Range<BranchId>) -> usize {
    // todo: maybe unchecked_sub?
    r.end.value() - r.start.value()
}

fn nth_child(range: Range<BranchId>, idx: usize) -> BranchId {
    let target = match range.start.value().checked_add(idx) {
        Some(v) => v,
        None => panic!(
            "the given child # ({idx}), when added to the child index start ({}), overflows usize",
            range.start.value()
        ),
    };
    let target = BranchId::new_branch(target);
    if !range.contains(&target) {
        panic!(
            "there are only {} children, but tried access #{idx}",
            range.end.value() - range.start.value()
        )
    }
    target
}

/// Mutable reference to a branch of [Ashes].
#[derive(Debug)]
pub struct BranchMut<'a, T> {
    // None for <root>
    node: Result<&'a mut Node<T>, RootInfo<'a>>,
}

impl<'a, T> BranchMut<'a, T> {
    /// Returns whether this is the root branch.
    pub fn is_root(&self) -> bool {
        self.node.is_err()
    }

    /// Returns the parent of this branch, or `None` if it is root.
    pub fn parent(&self) -> Option<BranchId> {
        match &self.node {
            Ok(node) => Some(node.parent),
            Err(_) => None,
        }
    }

    /// Returns the payload of this branch, or `None` if it is root.
    pub fn payload(&mut self) -> Option<&mut T> {
        match &mut self.node {
            Ok(node) => Some(&mut node.payload),
            Err(_) => None,
        }
    }

    /// Returns the range of IDs which are all children of this node.
    pub fn children(&self) -> Range<BranchId> {
        let range = match &self.node {
            Ok(node) => &node.children,
            Err(root) => root.children,
        };
        child_range(range)
    }

    /// Returns how many children this node has.
    pub fn children_len(&self) -> usize {
        children_len(self.children())
    }

    /// Returns the `n`th child's ID.
    pub fn child(&self, idx: usize) -> BranchId {
        nth_child(self.children(), idx)
    }
}

fn child_range(original: &Range<usize>) -> Range<BranchId> {
    BranchId::new_branch(original.start)..BranchId::new_branch(original.end)
}

#[derive(Debug, Clone)]
pub(crate) struct Node<T> {
    pub(crate) parent: BranchId,
    pub(crate) payload: T,
    pub(crate) children: Range<usize>,
    // note: do not rely on this being set!!
    //       the deserialization implementation simply sets this to usize::MAX
    //       this field is only meant for the `ForestFire::burn` impl
    pub(crate) old_idx: usize,
}

/// Immutable tree data structure.
///
/// While the tree's structure is immutable, the node values are fully available
/// through mutable references. For a mutable tree data structure, see [ForestFire].
///
/// `Ashes` may be serialized & deserialized; see [serde] (only available with the
/// `serde` feature enabled).
///
/// [ForestFire]: crate::fire::ForestFire
#[derive(Debug, Clone)]
pub struct Ashes<T> {
    pub(crate) nodes: Vec<Node<T>>,
    pub(crate) root_children: Range<usize>,
}

impl<T> Ashes<T> {
    /// Constructs a new, empty `Ashes<T>`.
    ///
    /// This is likely useless as `Ashes` cannot be inserted into, but some situations
    /// require it.
    pub const fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root_children: 0..0,
        }
    }

    /// Clears this tree.
    pub fn clear(&mut self) {
        self.root_children = 0..0;
        self.nodes.clear();
    }

    /// Checks whether there is a branch with the given branch ID.
    ///
    /// Branch IDs given out by an `Ashes` are valid for the entirety of that `Ashes`'s lifetime, so
    /// there is never a need to check if you're sure your branch ID came from this exact `Ashes`
    /// instance.
    ///
    /// Always returns `true` for [`BranchId::ROOT`].
    pub fn exists(&self, branch: BranchId) -> bool {
        if branch.is_root() {
            return true;
        }

        self.nodes.get(branch.value()).is_some()
    }

    /// Returns a shared reference to the root branch.
    pub fn root<'a>(&'a self) -> BranchRef<'a, T> {
        BranchRef {
            node: Err(RootInfo {
                children: &self.root_children,
            }),
        }
    }

    /// Returns a shared reference to the branch with the given ID.
    ///
    /// # Panics
    ///
    /// Panics if `branch` is not an [existing](Self::exists) branch.
    pub fn branch<'a>(&'a self, branch: BranchId) -> BranchRef<'a, T> {
        if branch.is_root() {
            self.root()
        } else {
            BranchRef {
                node: Ok(self
                    .nodes
                    .get(branch.value())
                    .unwrap_or_else(|| branch.indexing_panic())),
            }
        }
    }

    /// Returns a mutable reference to the root branch.
    pub fn root_mut<'a>(&'a mut self) -> BranchMut<'a, T> {
        BranchMut {
            node: Err(RootInfo {
                children: &self.root_children,
            }),
        }
    }

    /// Returns a mutable reference to the branch with the given ID.
    ///
    /// # Panics
    ///
    /// Panics if `branch` is not an [existing](Self::exists) branch.
    pub fn branch_mut<'a>(&'a mut self, branch: BranchId) -> BranchMut<'a, T> {
        if branch.is_root() {
            self.root_mut()
        } else {
            BranchMut {
                node: Ok(self
                    .nodes
                    .get_mut(branch.value())
                    .unwrap_or_else(|| branch.indexing_panic())),
            }
        }
    }

    /// Returns a range of root's child IDs.
    pub fn root_children(&self) -> Range<BranchId> {
        child_range(&self.root_children)
    }

    /// Returns an object which can be used to print the tree contents in a somewhat
    /// human-friendly format.
    ///
    /// The `print_value` function defines how the actual value is printed. In order of
    /// appearance, its parameters are:
    /// - `formatter: &mut fmt::Formatter`
    ///     - The formatter to write data into.
    /// - `payload: Option<&T>`
    ///     - The payload placed in the node currently being printed. `None` for root.
    /// - `depth: usize`
    ///     - The depth of the node within the tree. Starts at `0` for root.
    ///
    /// For values which already implement [`Debug`], see [`print_tree_debug`], or
    /// [`print_tree_display`] for [`Display`].
    ///
    /// [`print_tree_debug`]: #method.print_tree_debug
    /// [`print_tree_display`]: #method.print_tree_display
    pub fn print_tree<F: Fn(&mut fmt::Formatter, Option<&T>, usize) -> fmt::Result>(
        &self,
        print_value: F,
    ) -> PrintTree<'_, T, F> {
        PrintTree {
            ashes: self,
            print_value,
        }
    }

    /// Returns an object which can be used to print the tree contents in a somewhat
    /// human-friendly format.
    ///
    /// The values are printed using their [`Debug`] implementation; for further
    /// customization, see [`print_tree`].
    ///
    /// [`print_tree`]: #method.print_tree
    pub fn print_tree_debug(
        &self,
    ) -> PrintTree<'_, T, impl Fn(&mut fmt::Formatter, Option<&T>, usize) -> fmt::Result>
    where
        T: Debug,
    {
        PrintTree {
            ashes: self,
            print_value: |f, v, indent| {
                for _ in 0..(indent.saturating_mul(2)) {
                    write!(f, "-")?;
                }
                if let Some(v) = v {
                    writeln!(f, "{v:?}:")?;
                } else {
                    writeln!(f, "$:")?;
                }

                Ok(())
            },
        }
    }

    /// Returns an object which can be used to print the tree contents in a somewhat
    /// human-friendly format.
    ///
    /// The values are printed using their [`Display`] implementation; for further
    /// customization, see [`print_tree`].
    ///
    /// [`print_tree`]: #method.print_tree
    pub fn print_tree_display(
        &self,
    ) -> PrintTree<'_, T, impl Fn(&mut fmt::Formatter, Option<&T>, usize) -> fmt::Result>
    where
        T: Display,
    {
        PrintTree {
            ashes: self,
            print_value: |f, v, indent| {
                for _ in 0..(indent.saturating_mul(2)) {
                    write!(f, "-")?;
                }
                if let Some(v) = v {
                    writeln!(f, "{v}:")?;
                } else {
                    writeln!(f, "$:")?;
                }

                Ok(())
            },
        }
    }
}

impl<T> Default for Ashes<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// A struct for printing human-readable trees.
///
/// See [`Ashes::print_tree`].
pub struct PrintTree<'a, T, F: Fn(&mut fmt::Formatter, Option<&T>, usize) -> fmt::Result> {
    ashes: &'a Ashes<T>,
    print_value: F,
}

impl<'a, T, F: Fn(&mut fmt::Formatter, Option<&T>, usize) -> fmt::Result> PrintTree<'a, T, F> {
    /// Returns a reference to the [`Ashes`] instance used by this struct.
    pub fn ashes(&self) -> &'a Ashes<T> {
        self.ashes
    }
}

impl<'a, T, F: Fn(&mut fmt::Formatter, Option<&T>, usize) -> fmt::Result> Display
    for PrintTree<'a, T, F>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn print<T, F: Fn(&mut fmt::Formatter, Option<&T>, usize) -> fmt::Result>(
            ashes: &Ashes<T>,
            f: &mut std::fmt::Formatter<'_>,
            branch: BranchRef<T>,
            print_value: &F,
            indent: usize,
        ) -> fmt::Result {
            print_value(f, branch.payload(), indent)?;
            let sub_indent = indent.saturating_add(1);
            for child in branch.child_iter() {
                print(ashes, f, ashes.branch(child), print_value, sub_indent)?;
            }
            Ok(())
        }
        print(self.ashes, f, self.ashes.root(), &self.print_value, 0)
    }
}

impl<'a, T, F: Fn(&mut fmt::Formatter, Option<&T>, usize) -> fmt::Result> Debug
    for PrintTree<'a, T, F>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}
