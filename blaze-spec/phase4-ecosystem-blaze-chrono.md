# Phase 4 – Ecosystem Crate: `blaze‑chrono`

> **Goal:** Provide a comprehensive date and time library for Blaze, extending the core `std::time` module.  It offers timezone‑aware datetime types, formatting and parsing of common representations, calendar arithmetic, and integration with `blaze‑serde`.  All operations are pure unless loading timezone data from the file system.  The crate is designed to be the standard date/time library for Blaze applications that need more than `Instant` and `Duration`.

---

## 1. Core Types

### 1.1 `NaiveDateTime`

```
pub struct NaiveDateTime {
    pub date: NaiveDate,
    pub time: NaiveTime,
}
```

- A date and time without a timezone.  `@copy`.  Supports arithmetic, comparison, and formatting.

### 1.2 `NaiveDate`

```
pub struct NaiveDate { year: i32, ordinal: u16 }   // improved internal representation
```

- A date without a time.  Implements `Add<Duration>`, `Sub<Duration>`, `PartialOrd`, `Ord`, `Eq`.
- Methods: `year`, `month`, `day`, `weekday`, `from_ymd`, `from_ordinal`, `format`.

### 1.3 `NaiveTime`

```
pub struct NaiveTime { secs: u32, nanos: u32 }
```

- A time of day.  Range 00:00:00 – 23:59:59.999999999.

### 1.4 `DateTime<Tz: TimeZone>`

```
pub struct DateTime<Tz: TimeZone> {
    pub naive: NaiveDateTime,
    pub timezone: Tz,
}
```

- A timezone‑aware datetime.  Generic over the timezone type, allowing fixed offsets or IANA timezones.

---

## 2. Timezones

### 2.1 `Utc`

```
pub struct Utc;
impl TimeZone for Utc { … }
```

- The UTC timezone.

### 2.2 `FixedOffset`

```
pub struct FixedOffset { offset: i32 }   // seconds east of UTC
impl TimeZone for FixedOffset { … }
```

- A fixed offset from UTC.

### 2.3 `Local`

```
pub struct Local;
impl TimeZone for Local { … }
```

- The local system timezone, detected at runtime.

### 2.4 `IanaTimeZone` (optional, feature `iana`)

```
pub struct IanaTimeZone { id: Text, rules: TzRules }
impl TimeZone for IanaTimeZone { … }
```

- Full IANA timezone support (e.g., `"America/New_York"`).  The data is loaded from compiled zoneinfo files or embedded via the `@bundle` attribute.

---

## 3. Parsing and Formatting

### 3.1 `format`

```
impl NaiveDateTime {
    pub fn format(&self, fmt: &str) -> Text;
}
```

- Format specifiers follow strftime‑like placeholders (`%Y`, `%m`, `%d`, etc.).  Also supports easy constructors: `NaiveDateTime::parse(s: &str, fmt: &str) -> Result<NaiveDateTime, ChronoError>`.

### 3.2 ISO 8601

```
pub fn parse_iso8601(s: &str) -> Result<NaiveDateTime, ChronoError>;
pub fn to_iso8601(&self) -> Text;
```

- Convenience for the common ISO 8601 format.

---

## 4. Arithmetic and Duration

- `NaiveDateTime + Duration = NaiveDateTime`
- `NaiveDateTime - NaiveDateTime = Duration`
- `NaiveDateTime + Months = NaiveDateTime` (calendar arithmetic, handling month boundaries and leap years).

---

## 5. Serde Integration

All types implement `Serialize` and `Deserialize`, serializing to ISO 8601 strings for human‑readable formats, and to a compact binary representation for binary formats.

---

## 6. Error Handling

```
pub enum ChronoError {
    Parse(Text),
    OutOfRange,
    InvalidTimezone,
    Io(std::io::Error),
}
```

---

## 7. Testing

- **Construction:** Create a date, verify its weekday matches calendar.
- **Arithmetic:** Add a duration across a month boundary, verify correct rollover.
- **Parsing:** Parse an ISO 8601 string and compare with expected values.
- **Timezone conversion:** Convert a UTC time to a fixed offset and verify the offset.

All tests must pass on all platforms.
