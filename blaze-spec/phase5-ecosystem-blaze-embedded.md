# Phase 5 – Ecosystem Crate: `blaze‑embedded`

> **Goal:** Specify the `blaze‑embedded` crate, which provides the foundational layer for running Blaze on bare‑metal and resource‑constrained embedded systems (microcontrollers, SoCs, kernels).  It includes a `#![no_std]` runtime, a minimal panic handler, a configurable allocator for regions, and common abstractions for GPIO, timers, serial communication, and interrupt handling.  The crate is designed to be data‑oriented, zero‑cost, and deterministic, following Blaze’s core philosophy.  It serves as the base for more specific hardware abstraction layers (HALs) like `blaze‑cortex‑m` and `blaze‑esp32`.  All hardware interactions carry the `hal` effect, which implies `io`.

---

## 1. Core Concepts

Blaze on embedded targets runs without an operating system.  The `blaze‑embedded` crate provides:

- **`#![no_std]` and `#![no_runtime]`** – automatically applied when the crate is imported (via `@no_std` and `@no_runtime` attributes in the root module, or by simply depending on the crate).
- **A minimal panic handler** – by default, it halts the processor (`@panic("abort")`), but can be overridden to support logging or a custom fault handler.
- **A global allocator** – a configurable, region‑based allocator that works with the small memory regions typical on microcontrollers.  The allocator uses a simple bump‑pointer for static regions and a slab allocator for fixed‑size pools.
- **Hardware abstraction traits** – `GpioPin`, `Serial`, `Timer`, `InterruptController` – that must be implemented by specific HAL crates.
- **System tick and scheduling** – a cooperative scheduler based on the actor model (simplified for single‑threaded execution) and a `Sleep` future that yields to the next actor.

All I/O operations are tagged with the `hal` effect.

---

## 2. `#![no_std]` and `#![no_runtime]`

When the crate is linked, the compiler sets the `no_std` and `no_runtime` flags automatically (or the user can specify them in their root crate).  This means:

- The standard library prelude is not imported; only `core` (and a subset of `alloc`) are available.
- Memory allocation uses the crate’s own allocator, not the OS allocator.
- Panics trigger the default `panic_handler` defined by the crate, which calls a platform‑specific `halt` function (provided by the HAL).

---

## 3. Allocator

### 3.1 `Region` and `Arena`

Even in `no_std`, linear regions and arenas are the primary memory management strategy.  The crate provides a static bump‑pointer allocator that can be initialised with a fixed memory block (e.g., part of the device’s RAM).

```
pub struct StaticArena {
    start: *mut u8,
    end: *mut u8,
    current: AtomicPtr<u8>,
}
impl StaticArena {
    pub const fn new(start: *mut u8, size: usize) -> StaticArena;
    pub unsafe fn allocate(&self, layout: Layout) -> *mut u8;
    pub fn reset(&self);   // resets bump pointer to start (unsafe if memory in use)
}
```

- This allocator is lock‑free (using atomics) and suitable for single‑threaded use or with an interrupt‑safe mechanism.
- A global `ALLOCATOR` singleton is used by `Owned<T>`.
- `Pool<T>` from `std::mem` is also available, implemented on top of `StaticArena`.

### 3.2 `HeapAllocator`

Optionally, if the target has an external heap, the crate can hook into a `HeapAllocator` trait to provide `alloc` and `dealloc`, but the default is region‑based bump allocation.

---

## 4. Hardware Abstraction Traits

These traits define the interface that a specific HAL (e.g., `blaze‑cortex‑m`) must implement.  They are pure trait definitions inside `blaze‑embedded`.

### 4.1 `GpioPin`

```
pub trait GpioPin {
    fn set_high(&mut self);
    fn set_low(&mut self);
    fn toggle(&mut self);
    fn is_high(&self) -> bool;
    fn is_low(&self) -> bool;
    fn set_mode(&mut self, mode: PinMode);
}

pub enum PinMode { Input, Output, Alternate(u8), Analog }
```

### 4.2 `Serial`

```
pub trait Serial {
    fn write(&mut self, byte: u8) -> Result<(), SerialError>;
    fn read(&mut self) -> Result<u8, SerialError>;
    fn write_all(&mut self, data: &[u8]) -> Result<(), SerialError>;
    fn set_baud_rate(&mut self, baud: u32);
}

pub enum SerialError { Overrun, Parity, Framing, Noise, BufferFull, Timeout }
```

### 4.3 `Timer`

```
pub trait Timer {
    fn now(&self) -> u64;   // ticks since start
    fn delay_us(&mut self, us: u64);
    fn delay_ms(&mut self, ms: u64);
    fn set_alarm(&mut self, ticks: u64, callback: fn());
    fn clear_alarm(&mut self);
}
```

### 4.4 `InterruptController`

```
pub trait InterruptController {
    fn enable_irq(&self, irq: u16);
    fn disable_irq(&self, irq: u16);
    fn set_priority(&self, irq: u16, priority: u8);
    fn pending_irq(&self) -> Option<u16>;
    fn end_of_interrupt(&self, irq: u16);
}
```

---

## 5. Cooperative Actor Scheduler

The crate provides a simple, single‑threaded, cooperative scheduler that runs actors without pre‑emption.  It is suitable for embedded event loops.

```
pub struct Scheduler {
    actors: Vec<SpawnedActor>,
}

impl Scheduler {
    pub fn new() -> Scheduler;
    pub fn spawn<A: Actor>(&mut self, actor: A) -> Result<Capability<A>, SchedulerError>;
    pub fn tick(&mut self) -> bool;  // runs one step of the next ready actor; returns true if there are more actors
    pub fn run(&mut self) -> !;      // infinite loop calling tick
}
```

- Actors are spawned as they would be on a full Blaze runtime, but they run cooperatively.  An actor can suspend by calling `yield_now()` or `sleep(duration)` (which returns a future).
- The scheduler does not use pre‑emption or timers; it relies on actor voluntary yields.  A timer‑based wake can be implemented by the HAL’s `Timer` alarm if supported.

---

## 6. Panic Handler

The crate provides a default `#[panic_handler]` that calls a platform‑specific `halt()` function (exported by the HAL), which typically enters an infinite loop or resets the CPU.  It also optionally logs the panic message to the serial console if a `Serial` instance is registered globally.

```
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // if a serial console is registered, write the message
    // then call HAL’s halt()
}
```

The HAL must implement a `fn halt() -> !` function.

---

## 7. Startup and Linker Script

The crate provides a default `#[entry]` macro (if no HAL‑specific entry point is used) that initialises the allocator, sets up the stack pointer, and calls the user’s `main` function (which is an actor).  This is analogous to the `#[entry]` attribute in embedded Rust.

```
#[entry]
fn main() -> ! {
    // init allocator with memory region from linker script
    // create scheduler, spawn main actor, run
}
```

The linker script must define symbols `__heap_start` and `__heap_end` for the bump allocator.

---

## 8. Error Handling

```
pub enum EmbeddedError {
    OutOfMemory,
    InvalidAlignment,
    SerialError(SerialError),
    NoInterrupt,
    SchedulerFull,
    ActorPanicked,
}
```

---

## 9. Implementation Notes

- The crate uses `core` and `alloc` with the `alloc` feature enabled for `Vec`, `String` etc. via the crate’s allocator.  The `alloc` crate is provided by Blaze’s `sysroot` for `no_std` targets.
- All hardware traits are designed to be implemented behind `unsafe` blocks by HAL crates, but the API exposed to end users is safe.
- The allocator does not free individual blocks (bump allocator); memory is reclaimed only by resetting the arena or by explicit pool operations.  This matches Blaze’s region philosophy.
- The scheduler is simple but can be extended by a HAL to use hardware timers for efficient sleeping.

---

## 10. Testing

Testing embedded code requires either a device emulator (QEMU) or running tests on the host (with a mock HAL).  The crate provides a `mock` module behind a feature `mock‑hal` that implements all traits with software emulation, allowing unit tests on the host machine.

- **Allocator:** Allocate and use memory, verify bump pointer movement, reset.
- **GPIO mock:** Set pin high, assert high, toggle, check low.
- **Serial mock:** Write bytes, read back from a buffer, verify.
- **Scheduler:** Spawn multiple actors, tick, verify yield ordering.
- **Panic handler:** Trigger a panic, ensure the mock hal’s `halt()` is called.

All tests must pass using the mock HAL on the host CI.
