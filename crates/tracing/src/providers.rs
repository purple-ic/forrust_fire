//! Utilities for- and implementions of [`EventProvider`].
//!
//! Of interest:
//! - [`LogEventProvider`]: a built-in implementation of `EventProvider`.
//! - [`ProviderExt`]: some extensions for the `EventProvider` trait.
//!
//! [`LogEventProvider`]: log::LogEventProvider

use crate::{AshTrayce, EventProvider, run_forest_ret};

pub mod log;

/// Extensions for the [`EventProvider`] trait.
pub trait ProviderExt: EventProvider {
    /// Runs a function with a [`ForestFireSubscriber`] using the given [`EventProvider`],
    /// allowing returning values.
    ///
    /// This is just a shorthand for [`run_forest_ret`].
    ///
    /// [`ForestFireSubscriber`]: crate::ForestFireSubscriber
    ///
    /// # Panics
    ///
    /// See [`run_forest_ret`](crate::run_forest_ret#panics).
    fn run_ret<R>(self, func: impl FnOnce() -> R) -> (R, AshTrayce<Self>)
    where
        Self: Send,
        Self::Event: Send,
        Self: Sized,
    {
        run_forest_ret(self, func)
    }

    /// Runs a function with a [`ForestFireSubscriber`] using the given [`EventProvider`].
    ///
    /// This is just a shorthand for [`run_forest`].
    ///
    /// [`ForestFireSubscriber`]: crate::ForestFireSubscriber
    /// [`run_forest`]: crate::run_forest
    ///
    /// # Panics
    ///
    /// See [`run_forest_ret`](crate::run_forest_ret#panics).
    fn run(self, func: impl FnOnce()) -> AshTrayce<Self>
    where
        Self: Send,
        Self::Event: Send,
        Self: Sized,
    {
        let ((), trayce) = self.run_ret(func);
        trayce
    }
}

impl<P: EventProvider> ProviderExt for P {}
