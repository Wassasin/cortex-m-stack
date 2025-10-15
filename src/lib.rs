#![no_std]
#![doc = include_str!(concat!("../", env!("CARGO_PKG_README")))]

use core::{arch::asm, mem::size_of, ops::Range};

/// The value used to paint the stack.
pub const STACK_PAINT_VALUE: u32 = 0xCCCC_CCCC;

/// The [Range] currently in use for the stack.
///
/// Note: the stack is defined in reverse, as it runs from 'start' to 'end' downwards.
/// Hence this range is technically empty because `start >= end`.
///
/// If you want to use this range to do range-like things, use [stack_rev] instead.
#[inline]
pub const fn stack() -> Range<*mut u32> {
    unsafe extern "C" {
        static mut _stack_start: u32;
        static mut _stack_end: u32;
    }

    core::ptr::addr_of_mut!(_stack_start)..core::ptr::addr_of_mut!(_stack_end)
}

/// The [Range] currently in use for the stack,
/// defined in reverse such that [Range] operations are viable.
///
/// Hence the `end` of this [Range] is where the stack starts.
#[inline]
pub const fn stack_rev() -> Range<*mut u32> {
    stack().end..stack().start
}

/// Convenience function to fetch the current stack pointer.
#[inline]
pub fn current_stack_ptr() -> *mut u32 {
    let res;
    unsafe { asm!("mov {}, sp", out(reg) res) };
    res
}

/// The number of bytes that are reserved for the stack at compile time.
#[inline]
pub const fn stack_size() -> u32 {
    // Safety: start >= end. If this is not the case your linker did something wrong.
    (unsafe { stack().start.byte_offset_from_unsigned(stack().end) }) as u32
}

/// The number of bytes of the stack that are currently in use.
#[inline]
pub fn current_stack_in_use() -> u32 {
    // Safety: start >= end. If this is not the case your linker did something wrong.
    (unsafe { stack().start.byte_offset_from_unsigned(current_stack_ptr()) }) as u32
}

/// The number of bytes of the stack that are currently free.
///
/// If the stack has overflowed, this function returns 0.
#[inline]
pub fn current_stack_free() -> u32 {
    stack_size().saturating_sub(current_stack_in_use())
}

/// What fraction of the stack is currently in use.
#[inline]
pub fn current_stack_fraction() -> f32 {
    current_stack_in_use() as f32 / stack_size() as f32
}

/// Paint the part of the stack that is currently not in use.
///
/// **Note:** this can take some time, and an ISR could possibly interrupt this process,
/// dirtying up your freshly painted stack.
/// If you wish to prevent this, run this inside a critical section using [cortex_m::interrupt::free].
///
/// Runs in *O(n)* where *n* is the size of the stack.
/// This function is inefficient in the sense that it repaints the entire stack,
/// even the parts that still have the [STACK_PAINT_VALUE].
#[inline(never)]
pub fn repaint_stack() {
    unsafe {
        asm!(
            "0:",
            "cmp sp, r0",
            "bls 1f",
            "stmia r0!, {{r1}}",
            "b 0b",
            "1:",
            in("r0") stack().end,
            in("r1") STACK_PAINT_VALUE,
        )
    };
}

/// Finds the number of bytes that have not been overwritten on the stack since the last repaint.
///
/// In other words: shows the worst case free stack space since [repaint_stack] was last called.
///
/// Runs in *O(n)* where *n* is the size of the stack.
pub fn stack_painted() -> u32 {
    let res: *const u32;
    unsafe {
        asm!(
            "0:",
            "cmp sp, {ptr}",
            "bls 1f",
            "ldr {value}, [{ptr}]",
            "cmp {value}, {paint}",
            "bne 1f",
            "add {ptr}, #4",
            "b 0b",
            "1:",
            ptr = inout(reg) stack().end => res,
            value = out(reg) _,
            paint = in(reg) STACK_PAINT_VALUE,
            options(nostack, readonly)
        )
    };
    // Safety: res >= stack.end() because we start at stack.end()
    (unsafe { res.byte_offset_from_unsigned(stack().end) }) as u32
}

/// Finds the number of bytes that have not been overwritten on the stack since the last repaint using binary search.
///
/// In other words: shows the worst case free stack space since [repaint_stack] was last called.
///
/// Uses binary search to find the point after which the stack is written.
/// This will assume that the stack is written in a consecutive fashion.
/// Writing somewhere out-of-order into the painted stack will not be detected.
///
/// Runs in *O(log(n))* where *n* is the size of the stack.
///
/// **Danger:** if the current (active) stack contains the [STACK_PAINT_VALUE] this computation may be very incorrect.
///
/// # Safety
/// This function aliases the inactive stack, which is considered to be Undefined Behaviour.
/// Do not use if you care about such things.
pub unsafe fn stack_painted_binary() -> u32 {
    // Safety: we should be able to read anywhere on the stack using this,
    // but this is considered UB because we are aliasing memory out of nowhere.
    // Will probably still work though.
    let slice = unsafe {
        &*core::ptr::slice_from_raw_parts(stack().end, current_stack_free() as usize / 4)
    };
    (slice.partition_point(|&word| word == STACK_PAINT_VALUE) * size_of::<u32>()) as u32
}
