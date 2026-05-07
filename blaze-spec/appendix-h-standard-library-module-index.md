# Appendix H – Standard Library Module Index

> **Status:** Normative.  This appendix provides an alphabetically sorted list of every public name exported by the Blaze standard library, together with the module where it is defined.  The index is auto‑generated from the compiler’s metadata and is reproduced here for reference.  Standard library items from Phases 3a, 3b, 3c, and (where applicable) 3d are included.

---

## H.1 Conventions

- Entries are formatted as `name → module_path`.
- Types, traits, functions, and macros are all listed together.
- If a name is re‑exported through multiple paths, all paths are listed.
- The index covers the normative standard library; ecosystem crates (Phases 4‑5) are not included.

## H.2 Index

```
abs → std::cmp, std::ops
Add → std::ops
AddAssign → std::ops
Arena → std::mem
Allocator → std::mem
ApproxEq → std::cmp
args → std::env
AsMut → std::ops
AsRef → std::ops
Binary → std::ops (trait for binary serialization)
BitAnd → std::ops
BitAndAssign → std::ops
BitOr → std::ops
BitOrAssign → std::ops
BitXor → std::ops
BitXorAssign → std::ops
bool → (primitive type)
Box → std::mem (Owned<T>)
Capability → std::sync
Channel → std::sync (Sender / Receiver types)
char → (primitive type)
Clone → std::clone
Command → std::process
const → (keyword, not in std)
continue → (keyword)
CString → std::ffi (if present)
Default → std::default
Debug → std::debug (re‑exports std::fmt::Debug)
defer → (keyword)
Display → std::fmt
Dispose → std::mem (built‑in trait)
Div → std::ops
DivAssign → std::ops
Duration → std::time
dyn → (keyword)
else → (keyword)
ensures → (contract attribute)
Error (fmt) → std::fmt (unit type)
Error (io) → std::io
ErrorKind → std::io
Eq → std::cmp
ExitStatus → std::process
extern → (keyword)
f16, f32, f64, f128 → (primitive types)
false → (literal)
File → std::io
FnDef → std::meta
fmt → std::fmt (module, not an item)
for → (keyword)
format! → std::builtins (macro)
From → std::ops
FromIterator → std::iter
FromRow → std::ops (trait)
FromValue → std::sql
Future → std::future
Gauge → std::metrics (if metrics is in std)
gpu → (effect)
Hash → std::hash
Hasher → std::hash
HashMap → std::collections (Map)
i8, i16, i32, i64, i128, isize → (primitive types)
if → (keyword)
impl → (keyword)
in → (keyword)
Instant → std::time
Into → std::ops
IntoIterator → std::iter
Iterator → std::iter
Key → std::collections (SlotMap key)
let → (keyword)
List → std::collections
loop → (keyword)
Map → std::collections
match → (keyword)
Metadata → std::io
mod → (keyword)
move → (keyword)
Mul → std::ops
MulAssign → std::ops
mut → (keyword)
Neg → std::ops
None → std::builtins (Option::None)
NonNull → std::mem
Not → std::ops
Ok → std::builtins (Result::Ok)
Option → std::builtins
Ord → std::cmp
Ordering → std::cmp
Output → std::process
Owned → std::mem
panic! → std::builtins (macro)
PartialEq → std::cmp
PartialOrd → std::cmp
Path → std::path
Poll → std::future
pool → (attribute)
print → std::fmt
println → std::fmt
pub → (keyword)
Range → std::builtins
Read → std::io
Receiver → std::sync
ref → (keyword)
region → (keyword)
Rem → std::ops
RemAssign → std::ops
requires → (contract attribute)
Result → std::builtins
return → (keyword)
RingBuf → std::collections
Send → std::marker (unsafe trait)
Sender → std::sync
seq → (keyword)
Shl → std::ops
ShlAssign → std::ops
Shr → std::ops
ShrAssign → std::ops
SizeConstraint → std::layout (if in std, otherwise not)
SlotMap → std::collections
Some → std::builtins (Option::Some)
Split → std::string
static → (keyword)
Step → std::iter
str → (primitive type, behind reference)
String → std::string (alias for Text)
struct → (keyword)
Sub → std::ops
SubAssign → std::ops
Sync → std::marker (unsafe trait)
Text → std::string
Time → std::time (module, not item)
todo! → std::builtins (macro)
trait → (keyword)
true → (literal)
try → (keyword)
type → (keyword)
Type → std::meta
u8, u16, u32, u64, u128, usize → (primitive types)
union → (keyword)
unreachable! → std::builtins (macro)
unsafe → (keyword)
use → (keyword)
VarError → std::env
Vec → std::collections (or std::builtins, re‑exported)
Variant → std::meta
Version → std::env / std::meta (depending on context)
Wake → std::future
where → (keyword)
while → (keyword)
Window → std::window
WindowedView → std::window (compiler‑generated)
Windowable → std::window
Write → std::io
yield → (keyword)
```

## H.3 Notes

- This index is not exhaustive of every internal helper; it covers the public API surface required for conformance.
- The compiler generates a precise index in JSON format during the build of the standard library, which is used by `blaze doc` and the language server.  The above table is a human‑readable summary of that generated index.
