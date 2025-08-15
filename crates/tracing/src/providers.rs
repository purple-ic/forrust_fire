//! Utilities for- and implementions of [`EventProvider`].
//!
//! Of interest:
//! - [`LogEventProvider`]: a built-in implementation of `EventProvider`.
//! - [`ProviderExt`]: some extensions for the `EventProvider` trait.
//!
//! [`LogEventProvider`]: log::LogEventProvider

use crate::{AshTrayce, EventProvider, nothread_run_forest_ret};

pub mod log;

/// Extensions for the [`EventProvider`] trait.
// no need to seal this; since there is a blanket impl for any EventProvider _and_ the trait requires
//                       the implementor to be EventProvider, there is no way for someoneo to impl it
//                       on anything else
pub trait ProviderExt: EventProvider {
    /// Runs a function with a [`ForestFireSubscriber`] using the given [`EventProvider`],
    /// allowing returning values.
    ///
    /// This is just a shorthand for [`nothread_run_forest_ret`]. Just like `nothread_run_forest_ret`,
    /// the subscriber is **only set for the current thread**; if you'd like to multithread,
    /// consider a manual approach.
    ///
    /// [`ForestFireSubscriber`]: crate::ForestFireSubscriber
    ///
    /// # Panics
    ///
    /// See [`nothread_run_forest_ret`](crate::nothread_run_forest_ret#panics).
    fn nothread_run_ret<R>(self, func: impl FnOnce() -> R) -> (R, AshTrayce<Self>)
    where
        Self: Send,
        Self::Event: Send,
        Self: Sized,
    {
        nothread_run_forest_ret(self, func)
    }

    /// Runs a function with a [`ForestFireSubscriber`] using the given [`EventProvider`].
    ///
    /// This is just a shorthand for [`nothread_run_forest`]. Just like `nothread_run_forest`,
    /// the subscriber is **only set for the current thread**; if you'd like to multithread,
    /// consider a manual approach.
    ///
    /// [`ForestFireSubscriber`]: crate::ForestFireSubscriber
    /// [`nothread_run_forest`]: crate::run_forest
    ///
    /// # Panics
    ///
    /// See [`nothread_run_forest_ret`](crate::nothread_run_forest_ret#panics).
    fn run(self, func: impl FnOnce()) -> AshTrayce<Self>
    where
        Self: Send,
        Self::Event: Send,
        Self: Sized,
    {
        let ((), trayce) = self.nothread_run_ret(func);
        trayce
    }
}

impl<P: EventProvider> ProviderExt for P {}
