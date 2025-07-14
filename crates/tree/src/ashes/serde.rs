//! Tools for serializing & deserializing the [`Ashes`] data structure.
//!
//! For serializing:
//! - `Ashes` provides a [`Serialize`] implementation for payloads which also implement `Serialize`.
//! - `Ashes` also provides the [`Ashes::serializable_with`] method for custom serialization.
//!
//! For deserializing:
//! - `Ashes` provides a [`Deserialize`] implementation for payloads which also implement `Deserialize`.
//!     - This implementation will create temporary allocations required for building the tree. This
//!       should only be used for quick and hacky code, or if you are sure you will only ever deserialize
//!       a single tree.
//! - The [`AshDeserStorage`] structure allows reusing temporary buffer allocations and even supplying
//!   custom deserializers for the payload.

use std::{
    convert::identity,
    fmt::{Debug, Write},
    marker::PhantomData,
    ops::Range,
};

use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, DeserializeSeed, Error as _, Unexpected, Visitor},
    ser::SerializeMap,
};

use crate::{
    ashes::{Ashes, BranchId, BranchRef, Node},
    internal::serde::{ArrayFmt, USIZE_STR_MAX_CHARS},
};

// todo: different format for non-human-readable serializers

struct Ser<'a, T, S: Serialize, F: Copy + Fn(&'a T) -> S> {
    ashes: &'a Ashes<T>,
    branch: BranchRef<'a, T>,
    provider: F,
}

impl<'a, T, S: Serialize, F: Copy + Fn(&'a T) -> S> Serialize for Ser<'a, T, S, F> {
    fn serialize<SS>(&self, serializer: SS) -> Result<SS::Ok, SS::Error>
    where
        SS: Serializer,
    {
        let mut n = self.branch.n_children();
        if self.branch.payload().is_some() {
            n += 1;
        }

        let mut seq = serializer.serialize_map(Some(n))?;
        if let Some(payload) = self.branch.payload() {
            let payload = (self.provider)(payload);
            seq.serialize_entry("v", &payload)?;
        }
        for (i, child) in self.branch.child_iter().enumerate() {
            let mut arr = ArrayFmt::<USIZE_STR_MAX_CHARS>::new();
            write!(arr, "{i}")
                .expect("writing usize to sufficiently-sized buffer should never fail");
            seq.serialize_entry(
                arr.str(),
                &Ser {
                    ashes: self.ashes,
                    branch: self.ashes.branch(child),
                    provider: self.provider,
                },
            )?;
        }
        seq.end()
    }
}

impl<T: Serialize> Serialize for Ashes<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Ser {
            ashes: self,
            branch: self.root(),
            provider: identity,
        }
        .serialize(serializer)
    }
}

#[derive(Debug)]
struct Entry<T> {
    payload: T,
    children: Range<usize>,
}
// impl<T> Debug for Entry<T> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("Entry")
//             .field("payload", &..)
//             .field("children", &self.children)
//             .finish()
//     }
// }

/// A collection of buffers required for deserializing [Ashes].
///
/// A single `AshDeserStorage` may be used to deserialize multiple different
/// `Ashes`, reusing temporary allocations. The deserialized `Ashes` instance
/// can be simply [`std::mem::replace`]d or [`std::mem::take`]n out of the
/// `AshDeserStorage` instance.
///
/// # Examples
///
/// With `serde_json`:
/// ```
/// use forrust_fire_tree::ashes::serde::AshDeserStorage;
///
/// let mut storage = AshDeserStorage::<u32>::new();
/// let mut deserializer = serde_json::Deserializer::from_str(
/// r#"{
///     "1": {
///         "v": 3
///     },
///     "0": {
///         "v": 5
///     },
///     "2": {
///         "v": 0,
///         "0": {
///             "v": 1
///         }
///     }
/// }"#);
/// storage.deser(&mut deserializer).unwrap();
///
/// let ashes = std::mem::take(&mut storage.ashes);
/// // `ashes` now contains the tree
///
/// // check against the re-serialized version:
/// assert_eq!(
///     serde_json::to_value(&ashes).unwrap(),
///     serde_json::json!({
///         "0": {
///             "v": 5
///         },
///         "1": {
///             "v": 3
///         },
///         "2": {
///             "v": 0,
///             "0": {
///                 "v": 1
///             }
///         }
///     })
/// );
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub struct AshDeserStorage<T> {
    /// The [`Ashes`] instance into which the nodes will be written.
    ///
    /// This instance is cleared (without deallocating) whenever a new tree is
    /// deserialized. You are free to do anything with it (including taking it)
    /// inbetween deserializations.
    pub ashes: Ashes<T>,
    entry_stack: Vec<Option<Entry<T>>>,
}

impl<T> AshDeserStorage<T> {
    /// Creates a new, empty `AshDeserStorage`.
    pub fn new() -> Self {
        Self {
            ashes: Ashes::new(),
            entry_stack: Vec::new(),
        }
    }

    /// Creates a new deserialization seed using `Seed` for deserializing payloads.
    ///
    /// After deserialization, the output tree will be placed in [ashes].
    ///
    /// For `T`s which already implement `Deserialize`, it's simpler to use the [seed]
    /// method instead.
    ///
    /// Note that `Seed` will be copied excessively throughout the deserialization
    /// process; if `Seed` is expensive to clone, you should likely wrap it in an [`Rc`].
    ///
    /// [seed]: #method.seed
    /// [ashes]: #structfield.ashes
    pub fn seed_with<'de, 'a, Seed: DeserializeSeed<'de, Value = T> + Clone>(
        &'a mut self,
        seed: Seed,
    ) -> impl DeserializeSeed<'de, Value = ()> {
        self.ashes.clear();
        self.entry_stack.clear();

        let v: DeserSeed<'de, 'a, T, Seed, DeserRoot<T>> = DeserSeed {
            sub: seed,
            storage: self,
            phantom: PhantomData,
        };
        v
    }

    /// Creates a new deserialization seed.
    ///
    /// After deserialization, the output tree will be placed in [ashes].
    ///
    /// For `T`s which do not implement `Deserialize`, you'll have to use the [seed_with]
    /// method instead.
    ///
    /// [seed_with]: #method.seed_with
    /// [ashes]: #structfield.ashes
    pub fn seed<'de, 'a>(&'a mut self) -> impl DeserializeSeed<'de, Value = ()>
    where
        T: Deserialize<'de>,
    {
        self.seed_with(PhantomData::<T>)
    }

    /// Directly deserializes a tree using the given deserializer using [Seed] for
    /// deserializing payloads.
    ///
    /// After deserialization, the output tree will be placed in [ashes].
    ///
    /// For `T`s which already implement `Deserialize`, it's simpler to use the [deser]
    /// method instead.
    ///
    /// Note that `Seed` will be copied excessively throughout the deserialization
    /// process; if `Seed` is expensive to clone, you should likely wrap it in an [`Rc`].
    ///
    /// [ashes]: #structfield.ashes
    /// [deser]: #method.deser
    pub fn deser_with<
        'de,
        'a,
        Seed: DeserializeSeed<'de, Value = T> + Clone,
        Deser: Deserializer<'de>,
    >(
        &'a mut self,
        seed: Seed,
        deserializer: Deser,
    ) -> Result<(), Deser::Error> {
        self.seed_with(seed).deserialize(deserializer)
    }

    /// Directly deserializes a tree using the given deserializer
    ///
    /// After deserialization, the output tree will be placed in [ashes].
    ///
    /// For `T`s which do not implement `Deserialize`, you'll have to use the [deser_with]
    /// method instead.
    ///
    /// [deser_with]: #method.deser_with
    /// [ashes]: #field.ashes
    pub fn deser<'de, 'a, Deser: Deserializer<'de>>(
        &'a mut self,
        deserializer: Deser,
    ) -> Result<(), Deser::Error>
    where
        T: Deserialize<'de>,
    {
        self.deser_with(PhantomData::<T>, deserializer)
    }
}

impl<T> Default for AshDeserStorage<T> {
    fn default() -> Self {
        Self::new()
    }
}

trait DeserTy<T> {
    type Out;
    fn make_out<E: de::Error>(value: Option<T>) -> Result<Self::Out, E>;
    fn finish(#[allow(unused)] storage: &mut AshDeserStorage<T>) {}
}

struct DeserRoot<T>(PhantomData<T>);
impl<T> DeserTy<T> for DeserRoot<T> {
    type Out = ();

    fn make_out<E: de::Error>(value: Option<T>) -> Result<Self::Out, E> {
        match value {
            Some(_) => Err(E::custom("payload specified in root")),
            None => Ok(()),
        }
    }

    fn finish(storage: &mut AshDeserStorage<T>) {
        let start = storage.ashes.nodes.len();
        for entry in storage.entry_stack.drain(..) {
            let entry = entry.expect("root children should have been checked by now");

            let idx = storage.ashes.nodes.len();
            for child in Range::clone(&entry.children) {
                storage.ashes.nodes[child].parent = BranchId::new_branch(idx);
            }

            storage.ashes.nodes.push(Node {
                parent: BranchId::ROOT,
                payload: entry.payload,
                children: entry.children,
                old_idx: usize::MAX,
            });
        }
        let end = storage.ashes.nodes.len();
        storage.ashes.root_children = start..end;
    }
}
struct DeserChild<T>(PhantomData<T>);
impl<T> DeserTy<T> for DeserChild<T> {
    type Out = T;

    fn make_out<E: de::Error>(value: Option<T>) -> Result<Self::Out, E> {
        match value {
            Some(v) => Ok(v),
            None => Err(E::missing_field("v")),
        }
    }
}

struct DeserSeed<'de, 'a, T, Sub: DeserializeSeed<'de, Value = T> + Clone, Ty: DeserTy<T>> {
    sub: Sub,
    storage: &'a mut AshDeserStorage<T>,
    phantom: PhantomData<(&'de (), Ty)>,
}

impl<'de, 'a, T, Sub: DeserializeSeed<'de, Value = T> + Clone, Ty: DeserTy<T>> DeserializeSeed<'de>
    for DeserSeed<'de, 'a, T, Sub, Ty>
{
    type Value = Ty::Out;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(self)
    }
}

impl<'de, 'a, T, Sub: DeserializeSeed<'de, Value = T> + Clone, Ty: DeserTy<T>> Visitor<'de>
    for DeserSeed<'de, 'a, T, Sub, Ty>
{
    type Value = Ty::Out;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "a map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        #[derive(Debug, Clone, Copy)]
        enum Key {
            Payload,
            Child(usize),
        }
        struct KeyVisitor;
        impl<'de> Visitor<'de> for KeyVisitor {
            type Value = Key;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(
                    formatter,
                    "a string representing a number or the string 'v'"
                )
            }

            fn visit_str<E>(self, str: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if str == "v" {
                    Ok(Key::Payload)
                } else if let Ok(idx) = str.parse::<usize>() {
                    Ok(Key::Child(idx))
                } else {
                    Err(E::invalid_value(Unexpected::Str(str), &self))
                }
            }
        }
        impl<'de> Deserialize<'de> for Key {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer.deserialize_str(KeyVisitor)
            }
        }

        let mut payload = None;
        let start = self.storage.entry_stack.len();
        loop {
            let Some(key) = map.next_key::<Key>()? else {
                break;
            };
            match key {
                Key::Payload => {
                    if payload.is_some() {
                        return Err(A::Error::duplicate_field("v"));
                    }

                    payload = Some(map.next_value_seed(self.sub.clone())?);
                }
                Key::Child(i) => {
                    let sub_start = self.storage.entry_stack.len();
                    let sub: DeserSeed<'_, '_, _, _, DeserChild<T>> = DeserSeed {
                        sub: self.sub.clone(),
                        storage: self.storage,
                        phantom: PhantomData,
                    };
                    let child_payload = map.next_value_seed(sub)?;

                    // collect the sub-entry's entries
                    let sub_node_start = self.storage.ashes.nodes.len();
                    for child in self.storage.entry_stack.drain(sub_start..) {
                        let child = child
                            .expect("child part of entry stack should have been checked by now");
                        let node = Node {
                            parent: BranchId::UNINIT,
                            payload: child.payload,
                            children: child.children,
                            old_idx: usize::MAX,
                        };
                        let idx = self.storage.ashes.nodes.len();
                        for child in Range::clone(&node.children) {
                            self.storage.ashes.nodes[child].parent = BranchId::new_branch(idx);
                        }
                        self.storage.ashes.nodes.push(node);
                    }
                    let sub_node_end = self.storage.ashes.nodes.len();

                    let pos = start + i;
                    if self.storage.entry_stack.len() <= pos {
                        self.storage.entry_stack.resize_with(pos + 1, || None);
                    }
                    if self.storage.entry_stack[pos].is_some() {
                        return Err(A::Error::custom(format_args!("duplicate field `{i}`")));
                    }

                    self.storage.entry_stack[pos] = Some(Entry {
                        payload: child_payload,
                        children: sub_node_start..sub_node_end,
                    });
                }
            }
        }

        for (i, child) in self.storage.entry_stack[start..].iter().enumerate() {
            if child.is_none() {
                return Err(A::Error::custom(format_args!("missing field `{i}`")));
            }
        }

        let out = Ty::make_out(payload)?;
        Ty::finish(self.storage);
        Ok(out)
    }
}

impl<T> Ashes<T> {
    /// Returns a serializable object which uses the `provider` function to retrieve
    /// objects by which to serialize instances of `T`.
    ///
    /// `Ashes` itself implements `Serialize` for any `T` which also implements
    /// `Serialize`, so this method is likely not what you want unless you're
    /// implementing a custom serializer for `T`.
    pub fn serializable_with<'a, S, F>(&'a self, provider: F) -> impl Serialize + 'a
    where
        F: Copy + 'a + Fn(&'a T) -> S,
        S: Serialize + 'a,
    {
        Ser {
            ashes: self,
            branch: self.root(),
            provider,
        }
    }
}

impl<'de, T: Deserialize<'de> + 'de> Deserialize<'de> for Ashes<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut storage = AshDeserStorage::default();
        storage.deser(deserializer)?;
        Ok(storage.ashes)
    }
}
