# Disbelief

Rust interpreter for experimenting with coroutines.
It implements a closure compilation based tree walker, using the closure compilation step in order to resolve coroutine resume points at "compile" time. The tree walker uses continuation passing in order to capture the continuationn for the coroutine suspension.
