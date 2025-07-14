use std::fmt::{self, Debug};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BranchIdImpl {
    pub value: usize,
}

impl BranchIdImpl {
    pub const ROOT_STR: &'static str = "<root>";
    pub const UNINIT_STR: &'static str = "<uninit>";
    pub const ROOT: Self = Self { value: usize::MAX };
    pub const UNINIT: Self = Self {
        value: usize::MAX - 1,
    };

    pub const fn is_root(self) -> bool {
        self.value == usize::MAX
    }

    pub fn new_branch(value: usize) -> Self {
        debug_assert_ne!(value, usize::MAX, "branch ID should not be root");
        Self { value }
    }

    #[cold]
    pub fn indexing_panic(self) -> ! {
        panic!("given {self:?} does not point to any branch")
    }
}

impl Debug for BranchIdImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        struct FmtRoot;
        impl Debug for FmtRoot {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(BranchIdImpl::ROOT_STR)
            }
        }
        struct FmtUninit;
        impl Debug for FmtUninit {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(BranchIdImpl::UNINIT_STR)
            }
        }

        let mut t = f.debug_tuple("BranchId");
        if self.is_root() {
            t.field(&FmtRoot)
        } else if *self == Self::UNINIT {
            t.field(&FmtUninit)
        } else {
            t.field(&self.value)
        }
        .finish()
    }
}

macro_rules! define_branch_id {
    (
        $(#[$struct_attr:meta])*
        struct $ident:ident
    ) => {
        // too lazy to make these derives hygenic
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $(
            #[$struct_attr]
        )*
        pub struct $ident {
            pub(crate) value: $crate::internal::BranchIdImpl,
        }

        impl $ident {
            /// The branch ID representing the root node.
            pub const ROOT: Self = Self {
                value: $crate::internal::BranchIdImpl::ROOT,
            };
            #[allow(unused)] // only used in ashes::serde
            pub(crate) const UNINIT: Self = Self {
                value: $crate::internal::BranchIdImpl::UNINIT,
            };

            /// Returns whether this branch ID is [`ROOT`](Self::ROOT).
            pub const fn is_root(self) -> bool {
                self.value.is_root()
            }

            /// Returns the raw `usize` value behind this ID.
            pub const fn value(self) -> usize {
                self.value.value
            }

            /// Constructs a new ID from a raw `usize` value.
            pub const fn new(value: usize) -> Self {
                Self {
                    value: $crate::internal::BranchIdImpl { value },
                }
            }

            pub(crate) fn new_branch(value: usize) -> Self {
                Self {
                    value: $crate::internal::BranchIdImpl::new_branch(value),
                }
            }

            #[cold]
            pub(crate) fn indexing_panic(self) -> ! {
                self.value.indexing_panic()
            }
        }

        impl ::std::fmt::Debug for $ident {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                ::std::fmt::Debug::fmt(&self.value, f)
            }
        }
    };
}

#[cfg(feature = "serde")]
pub mod serde {
    use std::fmt::{self, Display};

    pub const USIZE_STR_MAX_CHARS: usize = usize::MAX.ilog10() as usize + 1;

    pub struct ArrayFmt<const N: usize> {
        // bytes in array[..cursor] must be valid utf-8
        // the Self::str impl converts it to `str` without
        // checking
        array: [u8; N],
        cursor: usize,
    }

    impl<const N: usize> fmt::Write for ArrayFmt<N> {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            let sub = &mut self.array[self.cursor..];
            if sub.len() < s.len() {
                Err(fmt::Error)
            } else {
                sub[..s.len()].copy_from_slice(s.as_bytes());
                self.cursor += s.len();

                Ok(())
            }
        }

        fn write_char(&mut self, c: char) -> fmt::Result {
            let len = c.len_utf8();
            let sub = &mut self.array[len..];
            if sub.len() < len {
                Err(fmt::Error)
            } else {
                c.encode_utf8(sub);
                self.cursor += len;
                Ok(())
            }
        }
    }

    impl<const N: usize> ArrayFmt<N> {
        pub const fn new() -> Self {
            Self {
                array: [0; N],
                cursor: 0,
            }
        }

        pub fn str(&self) -> &str {
            unsafe { str::from_utf8_unchecked(&self.array[..self.cursor]) }
        }
    }

    impl<const N: usize> Display for ArrayFmt<N> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            Display::fmt(self.str(), f)
        }
    }
}
