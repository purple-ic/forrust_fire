<!-- this is copied from the crate-level docs -->

Tracing integration for [`forrust_fire_tree`].

The main focus of this crate is [`ForestFireSubscriber`], which is able to build a tree of tracing
events & spans. Since it is based on [`ForestFire`] which allows inserting nodes into any part
of the tree, `ForestFireSubscriber` is capable of tracing multithreaded applications (note that
the state is behind a mutex, so some locking will be required!).

`ForestFireSubscriber` is generic over the data it collects from tracing events; the event
creation mechanism is specified by your [`EventProvider`] implementation, or you can simply
use the built-in `LogEventProvider` which will collect most of the data you'd likely want
from tracing (save for timing information).

This crate is part of the [`forrust_fire`](https://github.com/purple-ic/forrust_fire) collection.