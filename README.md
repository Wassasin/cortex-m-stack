# cortex-m-stack
Methods for Cortex-M processors to determine stack size and how much of the stack has been or is being used.

**Warning**: this crate depends on the `_stack_start` and `_stack_end` symbols being set correctly.
The `cortex-m` crates achieve this with their linker scripts, but the `flip-link` linker does not until [PR #126](https://github.com/knurling-rs/flip-link/pull/126) has landed.

## Immediate stack usage
Use [current_stack_in_use] or [current_stack_free] to keep track of the memory usage at run-time.

## Historical stack usage
First paint the stack using [repaint_stack] and then measure using [stack_painted] or [stack_painted_binary] to figure out how much stack was used between these two points.
