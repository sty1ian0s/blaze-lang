# Phase 5 – Ecosystem Crate: `blaze‑esp32`

> **Goal:** Specify the `blaze‑esp32` crate, which provides a hardware abstraction layer (HAL) for Espressif’s ESP32 series of microcontrollers (ESP32, ESP32‑S2, ESP32‑S3, ESP32‑C3, etc.).  It implements the traits defined in `blaze‑embedded` for the ESP32’s peripherals, including the Xtensa or RISC‑V core, GPIO, UART, SPI, I2C, ADC, PWM, Wi‑Fi, Bluetooth, and the FreeRTOS‑based firmware stack.  The crate is `#![no_std]` and uses raw register access wrapped in safe, linear types.  All hardware interaction carries the `hal` effect.  It integrates with `blaze‑embedded`’s cooperative scheduler and actor model, enabling reactive, actor‑based firmware with optional Wi‑Fi connectivity.

---

## 1. Core Concepts

The ESP32 family includes a rich set of peripherals and wireless connectivity.  The `blaze‑esp32` crate provides:

- **`Esp32`** – a context struct that holds all peripheral instances.
- **`GpioPin`** – implements `blaze‑embedded::GpioPin` for ESP32 GPIO pads.
- **`Uart`**, **`Spi`**, **`I2c`**, **`Adc`**, **`Pwm`** – implementations of the standard embedded traits using the ESP32’s peripheral registers.
- **`WiFi`** – a high‑level abstraction over the ESP32’s built‑in Wi‑Fi stack (using the Rust `esp‑wifi` or a similar embedded Wi‑Fi stack).  Wi‑Fi operations require the `hal` effect and are asynchronous (they use the actor model to handle events).
- **`Bluetooth`** – an optional abstraction for BLE classic/bluetooth (if supported by chip and feature `ble` is enabled).
- **`SystemTimer`** – implements `blaze‑embedded::Timer` using the ESP32’s timer group peripheral.
- **`InterruptController`** – implements `blaze‑embedded::InterruptController` for the ESP32’s interrupt matrix.

All peripherals are linear resources.  The crate relies on `core` and `alloc`, and uses a static bump allocator for Rust’s `alloc` crate (provided by `blaze‑embedded`’s default allocator, or a custom memory region).  The startup code initialises the heap, sets up the vector table, and calls the user’s `main` actor.

---

## 2. `Esp32` Context

### 2.1 Struct

```
pub struct Esp32 {
    pub gpio: GpioManager,
    pub uart0: Option<Uart>,
    pub uart1: Option<Uart>,
    pub spi0: Option<Spi>,
    pub i2c0: Option<I2c>,
    pub adc1: Option<Adc>,
    pub timer0: SystemTimer,
    pub intc: InterruptController,
    pub wifi: Option<WiFi>,
    pub bluetooth: Option<Bluetooth>,
}
```

- Created by `Esp32::new() -> Esp32` (or with a configuration block to enable only used peripherals to save resources).
- The struct owns all peripherals; moving it transfers ownership.  The user can destructure the context to take individual peripherals.

### 2.2 Initialization

`Esp32::new()` performs:

- Disabling watchdog timers (if configured).
- Setting up the interrupt matrix and enabling the CPU’s global interrupts.
- Initialising the timer group for `SystemTimer`.
- Optionally initialising the Wi‑Fi stack (will start the Wi‑Fi event task, see below).

---

## 3. GPIO

The ESP32 has up to 40 GPIO pads, each with flexible I/O matrix routing.  The crate provides `GpioManager` that owns all pads, and `GpioPin` for a single pad.

### 3.1 `GpioManager`

```
pub struct GpioManager {
    // private
}

impl GpioManager {
    pub fn pin(&mut self, number: u8, mode: PinMode) -> Option<GpioPin>;
    pub fn release(&mut self, pin: GpioPin) -> u8;   // returns pad number, pad available again
}
```

- `pin` takes a pad number (0‑39) and returns a `GpioPin` if the pad is not already taken.  Optionally configures internal pull‑up/pull‑down.
- The manager uses a bit‑mask to track allocated pads.  On `release`, the pad can be reused.

### 3.2 `GpioPin`

```
pub struct GpioPin {
    pad: u8,
}
```

Implements `GpioPin` trait methods using the GPIO matrix registers (set `GPIO_OUT_REG`/`GPIO_IN_REG`, etc.).  The pin’s ownership is linear; it must be returned to the manager to be reused.

---

## 4. UART

The ESP32 has two UART controllers (UART0, UART1).  UART0 is often used for the console and firmware upload.  The crate provides `Uart` that implements `blaze::embedded::Serial`.

### 4.1 `Uart`

```
pub struct Uart {
    uart_num: u8,
}
```

- Created by `Uart::new(uart_num: u8, baud: u32, pins: (u8, u8), config: &UartConfig) -> Result<Uart, EspError>`.
- `uart_num` 0 or 1.
- `pins` specifies TX and RX GPIO numbers.
- `UartConfig` includes data bits, stop bits, parity, flow control.

Methods: `write`, `read`, `write_all`, `set_baud_rate` – all safe.

On drop, `Dispose` resets the UART to defaults (optional via `reset_on_drop` config).

---

## 5. SPI, I2C, ADC, PWM

Similar pattern: each has a constructor requiring specific peripheral index and pins, returns a linear handle, implements a trait from `blaze‑embedded`.

### 5.1 `Spi`

```
pub struct Spi { spi_num: u8 }
impl Spi {
    pub fn new(spi_num: u8, sck: u8, mosi: u8, miso: u8, config: &SpiConfig) -> Result<Spi, EspError>;
    pub fn transfer(&mut self, write_buf: &[u8], read_buf: &mut [u8]) -> Result<(), EspError>;
}
```

- Supports SPI master mode.  Config includes clock speed, mode (CPOL/CPHA), bit order.

### 5.2 `I2c`

```
pub struct I2c { i2c_num: u8 }
impl I2c {
    pub fn new(i2c_num: u8, sda: u8, scl: u8, config: &I2cConfig) -> Result<I2c, EspError>;
    pub fn write_read(&mut self, addr: u8, write_buf: &[u8], read_buf: &mut [u8]) -> Result<(), EspError>;
}
```

- I2C master only.

### 5.3 `Adc`

```
pub struct Adc { adc_num: u8 }
impl Adc {
    pub fn new(adc_num: u8, pin: u8, config: &AdcConfig) -> Result<Adc, EspError>;
    pub fn read(&mut self) -> Result<u16, EspError>;
}
```

- ADC1 and ADC2 peripherals.  Supports attenuation, bit width.

### 5.4 `Pwm`

```
pub struct Pwm { timer: u8, channel: u8 }
impl Pwm {
    pub fn new(timer: u8, channel: u8, pin: u8, config: &PwmConfig) -> Result<Pwm, EspError>;
    pub fn set_duty(&mut self, duty: u16);
    pub fn set_frequency(&mut self, freq: u32);
    pub fn enable(&mut self);
    pub fn disable(&mut self);
}
```

- Uses the LEDC peripheral (LED PWM Controller) for flexible PWM on most GPIO pads.

---

## 6. Wi‑Fi

The ESP32’s built‑in Wi‑Fi stack is accessed via the `esp‑wifi` library.  The crate wraps it into an actor‑oriented `WiFi` type.

### 6.1 `WiFi`

```
pub struct WiFi {
    // internal handle to Wi‑Fi event actor
    sender: Sender<WifiCommand>,
    receiver: Receiver<WifiEvent>,
}
```

- Created by `WiFi::new(config: &WifiConfig) -> Result<WiFi, EspError>`.
- `WifiConfig` includes mode (Station, AccessPoint, or both), SSID, password, IP configuration, and optional static IP.
- `WiFi` is a linear resource that manages its own background actor to handle Wi‑Fi events.

### 6.2 Commands and Events

```
enum WifiCommand {
    Connect,
    Disconnect,
    StartScan,
    SetMode(WifiMode),
}

enum WifiEvent {
    Connected(IpInfo),
    Disconnected,
    ScanResult(Vec<AccessPoint>),
    Error(WifiError),
}
```

- The `WiFi` actor communicates with the Espressif Wi‑Fi stack (via `esp‑wifi` C bindings) and sends events to the application via a channel.

### 6.3 Convenience Methods

```
impl WiFi {
    pub async fn connect(&self);
    pub async fn disconnect(&self);
    pub async fn scan(&self) -> Result<Vec<AccessPoint>, WifiError>;
    pub async fn wait_for_connection(&self) -> Result<IpInfo, WifiError>;
    pub fn get_event_stream(&self) -> Receiver<WifiEvent>;
}
```

- All methods are async; they send commands and await responses from the Wi‑Fi actor.

---

## 7. Bluetooth (Optional, feature `ble`)

Similar pattern for BLE (Bluetooth Low Energy) using the ESP32’s Bluetooth stack.  Provides `Ble` struct with methods to start advertising, connect, send/receive data, etc.

---

## 8. System Timer

The ESP32 timer group (TIMG0/TIMG1) provides 64‑bit counters and alarms.  `SystemTimer` implements `blaze::embedded::Timer`.

```
pub struct SystemTimer { timer_num: u8 }
impl Timer for SystemTimer { … }
```

- Uses the internal 80 MHz APB clock divided for microsecond resolution.
- `delay_us` and `delay_ms` are blocking (busy‑wait).  For non‑blocking delays, use `set_alarm` and the actor’s sleep future.

---

## 9. InterruptController

Implements `blaze::embedded::InterruptController` using the ESP32’s interrupt matrix (allocating interrupts to CPU cores).  The controller handles both internal peripheral interrupts and external GPIO interrupts.

---

## 10. Startup and Linker Script

The crate provides a default `#[entry]` macro that:

- Defines the vector table (for Xtensa or RISC‑V).
- Initialises the heap (using the symbol `_heap_start` and `_heap_end` from the linker script).
- Sets up the Wi‑Fi event task if Wi‑Fi is used.
- Calls the user’s `main` actor.

The linker script must define the memory regions (IRAM, DRAM, etc.) and the heap boundaries.  The crate includes example linker scripts for common ESP32 dev boards.

---

## 11. Error Handling

```
pub enum EspError {
    InvalidPeripheral,
    InvalidPin,
    AlreadyInUse,
    RegisterAccess,
    Timeout,
    WifiError(WifiError),
    BleError(BleError),
    SerialError(SerialError),
    IoError,
}
```

- All peripheral operations return `Result<_, EspError>`.

---

## 12. Testing

Testing is primarily done on real hardware or via QEMU (the ESP32 has a QEMU fork).  A mock HAL can be used for unit tests.

- **GPIO:** Allocate a pin, set high, check state.
- **UART:** Write to UART, use external loopback to verify output.
- **Wi‑Fi:** Connect to a test Wi‑Fi network, verify IP assignment.
- **Timer:** Set alarm, wait for interrupt (simulated tick).
- **Actor integration:** Spawn an actor that uses Wi‑Fi; verify it can send and receive messages.

Tests are run on CI with real hardware or with a custom firmware simulator if available.
