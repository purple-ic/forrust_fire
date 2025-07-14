//! A built-in implementation of [`EventProvider`].
//!
//! The main point of interest here is [`LogEventProvider`].

use std::{fmt::Write as _, ops::Range};

use tracing::field;

use crate::{AshTrayce, EventInfo, EventProvider, Fields};

/// The finished-up and traversable tree returned by [`ForestFireSubscriber::burn`].
///
/// See [`AshTrayce`].
///
/// [`ForestFireSubscriber::burn`]: crate::ForestFireSubscriber::burn
pub type LogAshes = AshTrayce<LogEventProvider>;

/// Information for interpreting an event's field.
#[derive(Debug)]
#[non_exhaustive]
pub struct FieldInfo {
    // value will never naturally reach usize::MAX
    // because strings can be at most isize::MAX
    // bytes long
    /// The range of bytes within the [content string] which house the string
    /// representation of the field's value.
    ///
    /// If the field value has not yet been provided, this field will be set to
    /// `usize::MAX..usize::MAX`. The checking can be handled automatically for
    /// you by [`FieldInfo::value()`].
    ///
    /// [content string]: LogEventProvider::string
    pub value: Range<usize>,
    /// The field's name.
    pub name: &'static str,
}

impl FieldInfo {
    /// The range of bytes within the [content string] which house the string
    /// representation of the field's value, or `None` if it is not yet
    /// initialized.
    ///
    /// The range can be retrieved manually without checking for absence using
    /// the `value` field.
    ///
    /// [content string]: LogEventProvider::string
    pub fn value(&self) -> Option<Range<usize>> {
        if self.value == (usize::MAX..usize::MAX) {
            None
        } else {
            Some(Range::clone(&self.value))
        }
    }

    /// Extracts the subslice of the given string which represents the field's
    /// value, or `None` if the field has not been filled out.
    ///
    /// # Panics
    ///
    /// Panics if this struct's `value` field cannot be used to index the given
    /// string (be it due to char boundaries or due to out-of-range access),
    /// which can easily happen if the given string is not the same one which
    /// was used for recording this field.
    pub fn get_value<'s>(&self, str: &'s str) -> Option<&'s str> {
        self.value().map(|range| &str[range])
    }
}

/// A logged event.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct LogEvent {
    /// The range within [`LogEventProvider::field_infos`] which houses the fields
    /// used by the event.
    pub fields: Range<usize>,
    /// The metadata for this event.
    pub metadata: &'static tracing::Metadata<'static>,
}

/// A built-in implementation of [`EventProvider`] which provides most traced
/// information you'd want.
///
/// # Example
///
/// ```
/// # fn main() -> Result<(), serde_json::Error> {
/// use forrust_fire_tracing::providers::log::{LogEventProvider, LogAshes};
/// use forrust_fire_tracing::providers::ProviderExt;
///
/// // the `run` method is part of `ProviderExt`
/// let log_ashes: LogAshes = LogEventProvider::new().run(|| {
///     use tracing::info;
///
///     info!("hello, world");
/// });
///
/// // the `LogAshes` can be serialized if the `serde` feature is enabled!
/// // here's an example with `serde_json`:
///
/// #[cfg(feature = "serde")]
/// {
///     let json = serde_json::to_string_pretty(&log_ashes)?;
///     println!("{json}");
/// }
/// # Ok(()) }
/// ```
pub struct LogEventProvider {
    /// Field infos for recorded events.
    ///
    /// You can find out where to index to find the info for a particular
    /// event by [`LogEvent::fields`].
    pub field_infos: Vec<FieldInfo>,
    /// Field value representations for recorded events.
    ///
    /// You can find out where to index to find the string for a particular
    /// event by [`FieldInfo::value`] (or use [`FieldInfo::get_value`])
    pub string: String,
}

impl LogEventProvider {
    /// Creates a new, empty `LogEventProvider`.
    pub const fn new() -> Self {
        Self {
            field_infos: Vec::new(),
            string: String::new(),
        }
    }

    fn make_visitor_impl(&mut self, event: &mut LogEvent) -> impl tracing::field::Visit {
        struct V<'a> {
            p: &'a mut LogEventProvider,
            fields: Range<usize>,
        }
        impl<'a> field::Visit for V<'a> {
            fn record_debug(&mut self, field: &field::Field, value: &dyn std::fmt::Debug) {
                let str_start = self.p.string.len();
                self.p
                    .string
                    .write_fmt(format_args!("{value:?}"))
                    .unwrap_or_else(|_| todo!());
                let str_end = self.p.string.len();
                if field.index() >= self.fields.len() {
                    todo!()
                }
                let field_idx = self
                    .fields
                    .start
                    .checked_add(field.index())
                    .unwrap_or_else(|| todo!());
                self.p.field_infos[field_idx].value = str_start..str_end;
            }
        }
        V {
            p: self,
            fields: Range::clone(&event.fields),
        }
    }
}

impl Default for LogEventProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl EventProvider for LogEventProvider {
    type Event = LogEvent;

    #[inline]
    fn should_use_visitor_if_values_given(&self) -> bool {
        false
    }

    fn make_event(&mut self, _: usize, info: EventInfo) -> Self::Event {
        let field_start = self.field_infos.len();
        let iter = match info.fields {
            Fields::Full(field_set) => {
                self.field_infos.reserve(field_set.len());
                field_set.iter()
            }
            Fields::Iter(iter) => iter,
        };
        for field in iter {
            self.field_infos.push(FieldInfo {
                value: usize::MAX..usize::MAX,
                name: field.name(),
            });
        }
        let field_end = self.field_infos.len();

        let mut event = LogEvent {
            fields: field_start..field_end,
            metadata: info.metadata,
        };
        if let Some(values) = info.values_early {
            values.record(&mut self.make_visitor_impl(&mut event));
        }

        event
    }

    fn make_visitor(&mut self, _: usize, event: &mut Self::Event) -> impl tracing::field::Visit {
        self.make_visitor_impl(event)
    }

    #[inline]
    fn should_span_enter() -> bool {
        false
    }

    #[inline]
    fn should_span_exit() -> bool {
        false
    }
}

#[cfg(feature = "serde")]
mod serde {
    use std::ops::Range;

    use serde::{ser::SerializeMap, Serialize};
    use tracing_serde::{AsSerde, SerializeLevel};

    use crate::providers::log::{LogAshes, LogEvent};

    struct SerializeEventCtx<'a> {
        ashes: &'a LogAshes,
        event: &'a LogEvent,
    }

    impl<'a> Serialize for SerializeEventCtx<'a> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let mut map = serializer.serialize_map(Some(self.event.fields.len()))?;
            for field_idx in Range::clone(&self.event.fields) {
                let info = &self.ashes.provider.field_infos[field_idx];
                map.serialize_entry(info.name, &info.get_value(&self.ashes.provider.string))?;
            }
            map.end()
        }
    }

    #[derive(Serialize)]
    struct SerializeEvent<'a> {
        name: &'static str,
        target: &'a str,
        level: SerializeLevel<'a>,
        #[serde(rename = "mod")]
        module_path: Option<&'a str>,
        file: Option<&'a str>,
        line: Option<u32>,
        is_span: bool,
        ctx: SerializeEventCtx<'a>,
    }

    impl Serialize for LogAshes {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            self.ash
                .serializable_with(|event| {
                    let metadata = event.metadata;
                    SerializeEvent {
                        ctx: SerializeEventCtx { ashes: self, event },
                        name: metadata.name(),
                        target: metadata.target(),
                        level: metadata.level().as_serde(),
                        module_path: metadata.module_path(),
                        file: metadata.file(),
                        line: metadata.line(),
                        is_span: metadata.is_span(),
                    }
                })
                .serialize(serializer)
        }
    }
}
