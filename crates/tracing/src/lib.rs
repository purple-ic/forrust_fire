//! Tracing integration for [`forrust_fire_tree`].
//!
//! The main focus of this crate is [`ForestFireSubscriber`], which is able to build a tree of tracing
//! events & spans. Since it is based on [`ForestFire`] which allows inserting nodes into any part
//! of the tree, `ForestFireSubscriber` is capable of tracing multithreaded applications (note that
//! the state is behind a mutex, so some locking will be required!).
//!
//! `ForestFireSubscriber` is generic over the data it collects from tracing events; the event
//! creation mechanism is specified by your [`EventProvider`] implementation, or you can simply
//! use the built-in [`LogEventProvider`] which will collect most of the data you'd likely want
//! from tracing (save for timing information).
//!
//! [`LogEventProvider`]: crate::providers::log::LogEventProvider

#![warn(missing_docs)]

use std::{
    num::NonZeroU64,
    sync::{Arc, Mutex, MutexGuard},
    thread::{self, ThreadId},
};

use forrust_fire_tree::{
    ashes::Ashes,
    fire::{self, ForestFire},
};
use thread_local::ThreadLocal;
use tracing::{
    Subscriber,
    field::{self, ValueSet},
    span,
};

pub mod providers;
#[cfg(test)]
mod test;

/// A collection of fields, either provided through a full [`field::FieldSet`] or through
/// [`field::Iter`].
#[derive(Debug)]
pub enum Fields<'a> {
    /// The field information is provided fully.
    Full(&'a field::FieldSet),
    /// The field information is provided through an iterator.
    Iter(field::Iter),
}

/// Information about an event that has just started.
///
/// This is used by [`EventProvider::make_event`]. More fields may be added
/// in the future, so it is marked as `#[non_exhaustive]`.
#[derive(Debug)]
#[non_exhaustive]
pub struct EventInfo<'a> {
    /// If `true`, this span is a span. If `false`, it is an event.
    pub is_span: bool,
    /// The fields of this event.
    pub fields: Fields<'a>,
    /// The metadata of this event.
    pub metadata: &'static tracing::Metadata<'static>,
    /// The value set of the event, provided early.
    ///
    /// This is only sometimes available, and such is wrapped in an `Option`. Furthermore, if you use it,
    /// you'll likely want to override [`EventProvider::should_use_visitor_if_values_given`] to return
    /// `false`, otherwise your [visitor] will be called with a duplicate of this information.
    ///
    /// [visitor]: EventProvider::make_visitor
    pub values_early: Option<&'a ValueSet<'a>>,
}

/// Definition of how to create and manage events captured by [`ForestFireSubscriber`].
///
/// See [`LogEventProvider`] for an implementation which is provided by default.
///
/// If you're implementing this trait, please take a look at every function provided: a lot
/// of features are provided by default that you can disable to improve performance. (mainly
/// look out for `should_*` functions).
///
/// [`LogEventProvider`]: crate::providers::log::LogEventProvider
pub trait EventProvider: 'static {
    /// The type representing the events being captured.
    type Event: Sized;

    /// Create an event.
    fn make_event(&mut self, id: usize, info: EventInfo) -> Self::Event;

    /// Called whenever a span is entered.
    ///
    /// The default implementation does nothing.
    ///
    /// If you're not going to override this function, you should likely override
    /// [`EventProvider::should_span_enter`] to prevent unneeded mutex locking.
    #[inline]
    fn span_enter(&mut self, id: usize, event: &mut Self::Event) {
        let _ = (id, event);
    }

    /// Whether to call [`EventProvider::span_enter`].
    ///
    /// If this is enabled, the tree mutex is locked whenever a span is entered. If
    /// you do not have any custom logic in `span_enter`, you should likely override
    /// this and return `false`. The default implementation returns `true`.
    #[inline]
    fn should_span_enter() -> bool {
        true
    }

    /// Called whenever a span is exited.
    ///
    /// The default implementation does nothing.
    ///
    /// If you're not going to override this function, you should likely override
    /// [`EventProvider::should_span_exit`] to prevent unneeded mutex locking.
    #[inline]
    fn span_exit(&mut self, id: usize, event: &mut Self::Event) {
        let _ = (id, event);
    }

    /// Whether to call [`EventProvider::span_exit`].
    ///
    /// If this is enabled, the tree mutex is locked whenever a span is exited. If
    /// you do not have any custom logic in `span_exit`, you should likely override
    /// this and return `false`. The default implementation returns `true`.
    #[inline]
    fn should_span_exit() -> bool {
        true
    }

    /// Create a [visitor] to do something with an event's fields.
    ///
    /// The default implementation returns a visitor which does not do anything. If
    /// you're not going to override this method with something more useful, you should
    /// likely override [`EventProvider::should_use_visitor`]. Because visitors are
    /// passed through `&dyn`s, the noop behaviour of the default return value may not
    /// be noticed by the compiler, creating code which enumerates over the fields and
    /// then executes a noop function, simply wasting time.
    ///
    /// [visitor]: field::Visit
    #[inline]
    fn make_visitor(&mut self, id: usize, event: &mut Self::Event) -> impl field::Visit {
        let _ = (id, event);

        struct VisitNoop;
        impl field::Visit for VisitNoop {
            #[inline]
            fn record_debug(&mut self, _: &field::Field, _: &dyn std::fmt::Debug) {}
        }
        VisitNoop
    }

    /// Whether to call [`EventProvider::make_visitor`].
    ///
    /// See `make_visitor` to understand its possible performance impact. If you do
    /// not have any custom logic in `make_visitor`, you should likely override this
    /// and return `false`. The default implementation returns `true`.
    #[inline]
    fn should_use_visitor() -> bool {
        true
    }

    /// Whether to call [`EventProvider::make_visitor`] if the values have already
    /// been provided in [`EventInfo::values_early`].
    ///
    /// The default implementation delegates to [`EventProvider::should_use_visitor`].
    fn should_use_visitor_if_values_given(&self) -> bool {
        Self::should_use_visitor()
    }
}

// note: we assume that a Local is fine to read even if it's been poisoned
//       keep that in mind if we add any more functionality
struct Local {
    stack: Vec<fire::BranchId>,
    last_using_thread: ThreadId,
}

impl Default for Local {
    fn default() -> Self {
        Self {
            stack: Default::default(),
            last_using_thread: thread::current().id(),
        }
    }
}

// note: we assume that an Inner is fine to read even if it's been poisoned
//       keep that in mind if we add any more functionality
struct Inner<P: EventProvider> {
    forest: ForestFire<P::Event>,
    provider: P,
}

/// An implementation of [`tracing::Subscriber`] which records the structured tracing data
/// into an equally structured tree.
///
/// To actually run some code using this as the default tracing subscriber, see [`run_forest`]
/// and [`ProviderExt::run`].
///
/// See the [crate documentation](crate) for more information.
///
/// [`ProviderExt::run`]: providers::ProviderExt::run
pub struct ForestFireSubscriber<P: EventProvider> {
    inner: Mutex<Inner<P>>,
    stack: ThreadLocal<Mutex<Local>>,
}

impl<P: EventProvider> ForestFireSubscriber<P> {
    /// Creates a new tracing subscriber.
    ///
    /// The provided `forest` tree will not be cleared, and new nodes will
    /// be added starting from root.
    pub const fn new(forest: ForestFire<P::Event>, provider: P) -> Self {
        Self {
            inner: Mutex::new(Inner {
                forest,
                provider,
                // string: String::new(),
                // field_infos: Vec::new(),
            }),
            stack: ThreadLocal::new(),
        }
    }

    /// Finishes up this tree.
    ///
    /// Please see [ForestFire::burn] for performance considerations.
    pub fn burn(self) -> AshTrayce<P> {
        let inner = mutex_into_inner_ignore_poison(self.inner);
        let ash = inner.forest.burn();
        AshTrayce {
            ash,
            provider: inner.provider, // string: inner.string,
                                      // field_infos: inner.field_infos,
        }
    }

    fn local<'this>(&'this self) -> MutexGuard<'this, Local> {
        let mut local = mutex_lock_ignore_poison(self.stack.get_or_default());
        let current = thread::current().id();
        if local.last_using_thread != current {
            local.last_using_thread = current;
            local.stack.clear();
        }
        local
    }

    fn inner<'this>(&'this self) -> MutexGuard<'this, Inner<P>> {
        mutex_lock_ignore_poison(&self.inner)
    }
}

impl<P: EventProvider + Default> Default for ForestFireSubscriber<P> {
    fn default() -> Self {
        Self::new(ForestFire::default(), P::default())
    }
}

fn br2sp(branch: fire::BranchId) -> span::Id {
    debug_assert!(!branch.is_root());

    let v = branch
        .value()
        .try_into()
        .ok()
        .and_then(|x: u64| x.checked_add(1))
        .map(|x| 
            // SAFETY: checked_add cannot overflow and we just added 1, so `x`
            //         will never be zero
            unsafe { NonZeroU64::new_unchecked(x)
        })
        .unwrap_or_else(|| 
            panic!(
                "ID overflow: cannot convert branch ID {} (max = {}) to span ID",
                branch.value(), 
                u64::MAX - 1 // since spans are one-indexed
            )
        );
    span::Id::from_non_zero_u64(v)
}

fn sp2br(span: &span::Id) -> fire::BranchId {
    let v = span.into_non_zero_u64().get() - 1; // wont underflow cause span is non-zero
    let v = usize::try_from(v)
        .unwrap_or_else(|_| panic!("ID overflow: cannot convert span ID {v} (after converting to zero-indexing) to branch ID (max = {})", usize::MAX));
    fire::BranchId::new(v)
}

fn ensure_normal<T>(forest: &ForestFire<T>, original: &span::Id, id: fire::BranchId) {
    if !forest.exists(id) {
        panic!("the provided span ({}) does not refer to an existing node", original.into_u64())
    }
    if id.is_root() {
        panic!("the provided span ({}) refers to root", original.into_u64());
    }
}

impl<P: EventProvider> Subscriber for ForestFireSubscriber<P> {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        // todo?
        true
    }

    fn new_span(&self, span: &span::Attributes<'_>) -> span::Id {
        let local = self.local();
        let mut inner = self.inner();
        let parent = local.stack.last().copied().unwrap_or(fire::BranchId::ROOT);
        let id = inner.forest.next_id();
        let event = inner.provider.make_event(
            id.value(),
            EventInfo {
                is_span: true,
                fields: Fields::Full(span.fields()),
                metadata: span.metadata(),
                values_early: Some(span.values()),
            },
        );
        inner.forest.branch(parent, event);
        // let (id, fields) = add(&mut inner, parent, Ok(span.fields()), span.metadata(), true);

        if inner.provider.should_use_visitor_if_values_given() {
            let inner = &mut *inner; // deref now to skip DerefMut and get smart borrow checker
            let payload = inner.forest.payload_mut(id);
            span.values()
                .record(&mut inner.provider.make_visitor(id.value(), payload));
        }

        br2sp(id)
    }

    fn record(&self, span: &span::Id, values: &span::Record<'_>) {
        if !P::should_use_visitor() {
            return;
        }

        let id = sp2br(span);
        let mut inner = self.inner();
        let inner = &mut *inner;
        ensure_normal(&inner.forest, span, id);
        let payload = inner.forest.payload_mut(id);
        values.record(&mut inner.provider.make_visitor(id.value(), payload));

        // let fields = Range::clone(&payload.fields);
        // values.record(&mut inner.visitor(fields));
    }

    fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {
        // todo
    }

    fn event(&self, event: &tracing::Event<'_>) {
        let local = self.local();
        let mut inner = self.inner();
        let id = inner.forest.next_id();
        let payload = inner.provider.make_event(
            id.value(),
            EventInfo {
                is_span: false,
                fields: Fields::Iter(event.fields()),
                metadata: event.metadata(),
                values_early: None,
            },
        );
        let parent = local.stack.last().copied().unwrap_or(fire::BranchId::ROOT);
        inner.forest.branch(parent, payload);
        // let (_, fields) = add(
        //     &mut inner,
        //     parent,
        //     Err(event.fields()),
        //     event.metadata(),
        //     false,
        // );
        if P::should_use_visitor() {
            let inner = &mut *inner;
            let payload = inner.forest.payload_mut(id);

            event.record(&mut inner.provider.make_visitor(id.value(), payload));
        }
        // event.record(&mut inner.visitor(fields));
    }

    fn enter(&self, span: &span::Id) {
        let br = sp2br(span);
        let mut local = self.local();
        local.stack.push(br);
        drop(local);
        if P::should_span_enter() {
            let mut inner = self.inner();
            ensure_normal(&inner.forest, span, br);
            let payload = inner.forest.payload_mut(br);
            self.inner().provider.span_enter(br.value(), payload);
        }
    }

    fn exit(&self, span: &span::Id) {
        let br = sp2br(span);
        let mut local = self.local();
        // todo: is this correct behaviour?
        while let Some(removed) = local.stack.pop() {
            if removed == br {
                break;
            }
        }
        drop(local);
        if P::should_span_exit() {
            let mut inner = self.inner();
            ensure_normal(&inner.forest, span, br);
            let payload = inner.forest.payload_mut(br);
            self.inner().provider.span_exit(br.value(), payload);
        }
    }
}

/// The finished-up and traversable tree returned by [`ForestFireSubscriber::burn`].
///
/// Also allows you to retrieve the provider instance.
#[non_exhaustive]
#[derive(Debug)]
pub struct AshTrayce<P: EventProvider> {
    /// The event tree.
    pub ash: Ashes<P::Event>,
    /// The event provider used to create the `ForestFireSubscriber`.
    pub provider: P,
}

/// Runs a function with a [`ForestFireSubscriber`] using the given [`EventProvider`],
/// allowing returning values.
///
/// For the duration of the function, a `ForestFireSubscriber` will be set as the
/// [default trace subscriber] for the current thread; any uses of the default trace
/// macros will report to this subscriber.
///
/// If you're not returning any values, you'll probably want [`run_forest`].
///
/// [default trace subscriber]: tracing::subscriber::with_default
///
/// # Panics
///
/// While the closure is being executed, the subscriber is actually wrapped in an [`Arc`].
/// Once the closure ends, the `Arc` is [unwrapped] and thus expected to have no living
/// references. Technically speaking, the code inside the closure could create a new
/// reference to that `Arc`, forcing the function to **panic**.
///
/// [unwrapped]: Arc::into_inner
pub fn nothread_run_forest_ret<Provider: EventProvider + Send, R>(
    provider: Provider,
    func: impl FnOnce() -> R,
) -> (R, AshTrayce<Provider>)
where
    Provider::Event: Send,
{
    let fire = ForestFireSubscriber::new(ForestFire::default(), provider);
    let fire = Arc::new(fire);
    let out = tracing::subscriber::with_default(Arc::clone(&fire), func);
    let fire = Arc::into_inner(fire).unwrap_or_else(|| panic!("forest fire escaped"));
    let ash = fire.burn();
    (out, ash)
}

/// Runs a function with a [`ForestFireSubscriber`] using the given [`EventProvider`].
///
/// For the duration of the function, a `ForestFireSubscriber` will be set as the
/// [default trace subscriber] for the current thread; any uses of the default trace
/// macros will report to this subscriber.
///
/// If you'd like to return a value from the closure, use [`run_forest_ret`].
///
/// [default trace subscriber]: tracing::subscriber::with_default
///
/// # Panics
///
/// See [`run_forest_ret`](run_forest_ret#panics)
pub fn nothread_local_run_forest<Provider: EventProvider + Send>(
    provider: Provider,
    func: impl FnOnce(),
) -> AshTrayce<Provider>
where
    Provider::Event: Send,
{
    let ((), trayce) = nothread_run_forest_ret(provider, func);
    trayce
}

fn mutex_lock_ignore_poison<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    // how i wish i could just have sync_nonpoison
    match mutex.lock() {
        Ok(v) => v,
        Err(err) => {
            mutex.clear_poison();
            err.into_inner()
        }
    }
}

fn mutex_into_inner_ignore_poison<T>(mutex: Mutex<T>) -> T {
    match mutex.into_inner() {
        Ok(x) => x,
        Err(e) => e.into_inner(),
    }
}