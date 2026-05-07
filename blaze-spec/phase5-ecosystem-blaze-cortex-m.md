# Phase 5 – Ecosystem Crate: `blaze‑cortex‑m`

> **Goal:** Specify the `blaze‑cortex‑m` crate, which provides a hardware abstraction layer (HAL) for ARM Cortex‑M microcontrollers (M0, M3, M4, M7, M33). It implements the traits defined in `blaze‑embedded` for Cortex‑M devices, including the NVIC interrupt controller, SysTick timer, GPIO, USART, SPI, I2C, ADC, and PWM peripherals via common register patterns. The crate is `#![no_std]` and uses raw register access (via `unsafe` blocks) wrapped in safe, linear types. All hardware interaction carries the `hal` effect. The crate is data‑oriented: each peripheral is a plain struct owning its register block, with no virtual dispatch. It is designed to be used with `blaze‑embedded`’s cooperative scheduler, enabling reactive, actor‑based firmware.

---

## 1. Core Concepts

The Cortex‑M architecture provides a standardised NVIC (Nested Vectored Interrupt Controller) and a SysTick timer, plus a common set of memory‑mapped peripherals that vary by chip family. This crate provides:

- **`CortexM`** – a context struct that holds the NVIC and SysTick handles.
- **`SysTick`** – implements `blaze‑embedded::Timer` using the 24‑bit SysTick counter.
- **`NVIC`** – implements `blaze‑embedded::InterruptController` for the standard system exceptions and external interrupts.
- **`GpioPort`** and **`GpioPin`** – implements `GpioPin` for the standard GPIO registers (ODR, IDR, MODER, etc. as found on STM32, similar patterns on other vendors).
- **`Serial`** – USART wrapper implementing `blaze‑embedded::Serial`.
- **`Spi`**, **`I2c`**, **`Adc`**, **`Pwm`** – additional peripheral abstractions.

All peripherals are linear resources: moving a `GpioPort` into a pin constructor consumes the port, ensuring only one owner per pin. The crate uses `@layout(C)` for register block structs to match the hardware layout exactly. All low‑level register reads/writes are unsafe, wrapped in short safe methods that maintain the borrow checker’s guarantees.

---

## 2. `CortexM` Context

### 2.1 Struct

```
pub struct CortexM {
    pub systick: SysTick,
    pub nvic: NVIC,
}
```

- Created by `CortexM::new() -> CortexM`. The `SysTick` and `NVIC` refer to the core peripherals at their standard memory addresses (`0xE000E010` for SysTick, `0xE000E100` for NVIC).

### 2.2 Core Peripheral Access

```
impl CortexM {
    pub fn systick(self) -> SysTick;
    pub fn nvic(self) -> NVIC;
    pub fn reset(&self);
    pub fn disable_interrupts() -> bool;  // returns prior state, used for critical sections
    pub fn enable_interrupts();
}
```

- `reset()` triggers a software system reset via the AIRCR register.

---

## 3. `SysTick` Timer

### 3.1 Struct

```
pub struct SysTick {
    base: *mut SysTickRegisters,
}
```

- `SysTickRegisters` is a `#[repr(C)]` struct with `csr`, `rvr`, `cvr`, `calib`.

### 3.2 `Timer` Trait Implementation

```
impl Timer for SysTick {
    fn now(&self) -> u64;               // current value of the cycle counter (24‑bit, but extended with overflow count)
    fn delay_us(&mut self, us: u64);    // busy‑wait delay
    fn delay_ms(&mut self, ms: u64);    // busy‑wait delay
    fn set_alarm(&mut self, ticks: u64, callback: fn());
    fn clear_alarm(&mut self);
}
```

- Uses the SysTick reload value to generate periodic interrupts. The callback is registered and called from the `SysTick` exception handler (set up at startup).

---

## 4. `NVIC` Interrupt Controller

### 4.1 Struct

```
pub struct NVIC {
    base: *mut NVICRegisters,
}
```

- `NVICRegisters` holds `iser`, `icer`, `ispr`, `icpr`, `iabr`, `ipr` arrays.

### 4.2 `InterruptController` Trait Implementation

```
impl InterruptController for NVIC {
    fn enable_irq(&self, irq: u16);
    fn disable_irq(&self, irq: u16);
    fn set_priority(&self, irq: u16, priority: u8);
    fn pending_irq(&self) -> Option<u16>;
    fn end_of_interrupt(&self, irq: u16);
}
```

- All methods directly manipulate the NVIC registers. The `end_of_interrupt` is a no‑op for Cortex‑M (the hardware handles EOI automatically), but kept for API compatibility.

---

## 5. GPIO

### 5.1 `GpioPort`

```
pub struct GpioPort {
    base: *mut GpioRegisters,
    port_letter: char,
}
```

- Created by a macro that maps chip‑specific peripheral addresses to `GpioPort` instances (e.g., `GpioPort::new('A')`).

- Methods:
  - `pub fn pin(&self, pin: u8, mode: PinMode) -> GpioPin;` – consumes the port’s ownership of that pin, returning a `GpioPin`.

### 5.2 `GpioPin`

```
pub struct GpioPin {
    port: GpioPort,
    pin: u8,
}
```

- Implements `blaze::embedded::GpioPin`:
  - `set_high`, `set_low`, `toggle`, `is_high`, `is_low`, `set_mode`.
- On drop, the pin is not automatically returned to the port; the pin remains owned until the application explicitly releases it (if needed, the user can store the pin and later convert back to port via a method, but typically pins are long‑lived).

### 5.3 `GpioRegisters`

`#[repr(C)]` struct with `MODER`, `OTYPER`, `OSPEEDR`, `PUPDR`, `IDR`, `ODR`, `BSRR`, `LCKR`, `AFRL`, `AFRH`. The exact layout matches the reference manual of the target family (e.g., STM32F4). For portability, the crate provides common methods that work across all Cortex‑M families; the chip‑specific address mapping is provided by the chip family crate (e.g., `blaze‑stm32f4` re‑exports these types with the correct base addresses).

---

## 6. Serial (USART)

### 6.1 `Usart`

```
pub struct Usart {
    base: *mut UsartRegisters,
    baud_rate: u32,
}
```

- Created by `Usart::new(usart: UsartPeripheral, baud_rate: u32, clock_freq: u32) -> Usart`.
- `UsartPeripheral` is an enum of available USART instances (USART1, USART2, …). Each has a known base address.

### 6.2 `Serial` Trait Implementation

```
impl Serial for Usart {
    fn write(&mut self, byte: u8) -> Result<(), SerialError>;
    fn read(&mut self) -> Result<u8, SerialError>;
    fn write_all(&mut self, data: &[u8]) -> Result<(), SerialError>;
    fn set_baud_rate(&mut self, baud: u32);
}
```

- Uses the status register (TXE, RXNE) and data register (DR). If the TX buffer is full, `write` blocks until space is available; similarly `read` blocks until data is received. In an actor system, this can be wrapped in an async task that yields while waiting.

### 6.3 `UsartRegisters`

`#[repr(C)]` struct with `SR`, `DR`, `BRR`, `CR1`, `CR2`, `CR3`, `GTPR` (STM32‑style). Interrupt‑driven operation is supported by enabling `RXNEIE` and providing an interrupt handler (via `NVIC`). The crate provides a `UsartRx` stream that can be polled.

---

## 7. SPI, I2C, ADC, PWM (Overview)

Each peripheral follows the same pattern: a `#[repr(C)]` register block struct, a linear owner struct, and implementation of a trait from `blaze‑embedded`.

- **`Spi`** – full‑duplex, master‑only. Methods: `transfer`, `write`, `read`. Implements a minimal `Spi` trait (not yet in `blaze‑embedded`, defined locally in this crate).
- **`I2c`** – master mode. Methods: `write`, `read`, `write_read`. Includes timeouts.
- **`Adc`** – single‑ended and differential inputs, configurable resolution (12/10/8/6 bits). Methods: `read`, `start_conversion`.
- **`Pwm`** – timer‑based PWM with duty cycle control. Methods: `set_duty`, `set_frequency`, `enable`, `disable`.

Each peripheral’s constructor takes a peripheral instance identifier (an enum with known base address) and configuration parameters. The constructors are `unsafe` because they access memory‑mapped I/O, but the resulting types are safe to use (as long as the chip family crate guarantees the base addresses and that no two instances alias).

---

## 8. Startup and Vector Table

The crate provides a default `#[entry]` macro (or a `startup.rs` file) that:

- Initialises the `.bss` and `.data` sections.
- Sets the vector table offset (VTOR) if relocatable.
- Calls the user’s `main` function (which is an actor).
- Defines the default exception handlers (e.g., `HardFault`, `SysTick`). The `HardFault` handler can be overridden to log diagnostic information via `Serial`.

The vector table is defined in a linker script that the user provides (typically `memory.x` and `layout.ld`). The crate re‑exports common linker‑script symbols.

---

## 9. Integration with `blaze‑embedded`

This crate implements the traits from `blaze‑embedded`. A typical embedded application structure:

```
@no_std
@no_runtime

use blaze::embedded::{scheduler, GpioPin, Serial, Timer};
use blaze::cortex_m::{CortexM, Usart, GpioPort};

struct MainActor;
impl Actor for MainActor {
    fn on_message(&mut self, msg: ()) {
        let ctx = CortexM::new();
        let mut led = GpioPort::new('B').pin(0, PinMode::Output);
        loop {
            led.toggle();
            ctx.systick.delay_ms(500);
        }
    }
}

#[entry]
fn main() -> ! {
    let mut sched = Scheduler::new();
    sched.spawn(MainActor).unwrap();
    sched.run();
}
```

---

## 10. Error Handling

```
pub enum CmError {
    InvalidPeripheral,
    RegisterAccess,
    Timeout,
    SerialError(SerialError),
    AdcOverrun,
    I2cNak,
    I2cBusError,
    SpiModeFault,
}
```

- All peripheral methods that can fail return `Result<_, CmError>`.

---

## 11. Testing

Testing is done via QEMU (for Cortex‑M3) or on real hardware. A mock HAL (`blaze‑embedded`’s mock module) can be used for logic testing.

- **GPIO:** Create a pin, set high, assert high via mock register read.
- **Serial:** Write bytes, use mock to simulate a loopback, read back.
- **SysTick:** Configure an alarm, manually tick the counter, verify callback is called.
- **NVIC:** Enable an IRQ, check pending after a simulated interrupt.
- **Interrupt handler:** Define a custom `SysTick` handler, verify it is invoked after a tick.

Tests for the actual register manipulation are run in QEMU by loading a small firmware that exercises each peripheral and checks expected values. These are in a separate `qemu-tests` directory.
