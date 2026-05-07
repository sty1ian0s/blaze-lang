# Phase 5 – Tooling: `blaze‑migrate`

> **Goal:** Provide an automatic source‑to‑source migration tool that converts object‑oriented codebases (targeting Java and C# initially) into idiomatic Blaze.  The tool reads the original source files, analyses their structure, and produces a set of `.blz` files where classes are replaced by plain `@data` structs, inheritance by composition and `enum`‑based dispatch, interfaces by traits, and patterns like virtual dispatch by `dyn Trait` or `match` over enum variants.  The migration is safe (the resulting Blaze code compiles if the original compiled) and preserves all logic, while eliminating the OOP patterns that Blaze rejects.

## 1. Scope and Limitations

`blaze‑migrate` handles:

- Class definitions → structs with all fields flattened.
- Single and multiple inheritance (via interfaces) → composition and `dyn Trait`.
- Virtual method dispatch → `match` over an enum of concrete types, or `dyn Trait` where dynamic dispatch is truly needed.
- Getters, setters, and constructor boilerplate → `@data` and `@newtype` attributes.
- Collections (Java `ArrayList`, C# `List<T>`, etc.) → `Vec<T>` and `Map<K,V>` with appropriate `use` statements.
- Nullable references → `Option<T>`.
- Exceptions → `Result<T, E>` with proper error propagation.
- Simple synchronized blocks → actor messages.

`blaze‑migrate` does **not** handle:

- Highly reflective code (runtime type introspection) that cannot be replaced by compile‑time features.
- Platform‑specific GUI code (e.g., Android SDK, WinForms) – these require manual rewriting.
- Code that relies on deeply mutable global state without clear ownership – such code must be refactored manually before migration, or left in `unsafe` blocks during migration.

The tool is designed to produce **idiomatic Blaze**, not just syntactically valid Blaze.  The output is immediately usable and, after optional manual tweaking, ready for AOT compilation.

## 2. Input and Output

### 2.1 Input

`blaze‑migrate` reads an existing project tree (Maven/Gradle for Java, `.csproj` for C#) and automatically discovers source files, dependencies, and configuration.

### 2.2 Output

The tool generates in a new directory:

- One `.blz` file per original class, placed in the corresponding module hierarchy.
- A `blaze.toml` manifest with the appropriate dependencies on Blaze ecosystem crates (e.g., `blaze‑sql` if the original used JDBC).
- A `README.md` with a summary of decisions made and any manual steps recommended.

## 3. Migration Algorithm

The algorithm runs in five phases.  Each phase transforms an intermediate representation (IR) of the original program.

### 3.1 Phase 1 – Parsing and Analysis

The tool uses the original language’s front‑end (e.g., `javaparser` for Java, Roslyn for C#) to parse every source file and build a complete symbol table.  It resolves all types, method calls, and control flow to the extent that the original compiler would.

### 3.2 Phase 2 – Inheritance Flattening

For every class hierarchy, the tool:

1. Collects all fields from the class and all its ancestors into a single struct.
2. If the class has sub‑classes that override methods, an `enum` is created whose variants are the concrete sub‑classes.  Each variant holds the fields unique to that sub‑class.
3. Methods that are not overridden become inherent `fn` on the struct or enum.
4. Overridden methods become functions that `match` on the enum variant, or – if the hierarchy is open for external extension – a `dyn Trait` is generated.

### 3.3 Phase 3 – Interface Conversion

Original interfaces become Blaze `trait`s.  Classes that implement an interface generate an `impl Trait for Type` block.  Dependencies that expected an interface type are converted to `&dyn Trait` or `impl Trait` parameters, depending on whether dynamic dispatch is required.

### 3.4 Phase 4 – Resource Management

- `try`/`finally` blocks for resource cleanup are converted to `with` blocks (which use `Dispose`).
- Objects that were explicitly closed or disposed become linear types that implement `Dispose`.
- Where the tool cannot prove linear usage, it conservatively wraps the value in `Rc` and emits a warning suggesting the developer refactor for linear ownership.

### 3.5 Phase 5 – Clean‑Up and Formatting

- Redundant wrapper methods (getters/setters) are removed and replaced with public field access, or `@newtype` attributes.
- The generated Blaze source is run through `blaze fmt` and `blaze fix --aot-ify`.
- Unused imports are removed.
- A `@migrated` attribute is placed on every top‑level item, containing a comment with the original file and line number, aiding manual review.

## 4. Example: Java to Blaze

**Original Java:**

```java
public abstract class Animal {
    private String name;
    public Animal(String name) { this.name = name; }
    public String getName() { return name; }
    public abstract String speak();
}

public class Dog extends Animal {
    public Dog(String name) { super(name); }
    @Override
    public String speak() { return "Woof!"; }
}

public class Cat extends Animal {
    public Cat(String name) { super(name); }
    @Override
    public String speak() { return "Meow!"; }
}
```

**Migrated Blaze:**

```blaze
@data
struct Animal {
    name: String,
}

enum AnimalKind {
    Dog(Animal),
    Cat(Animal),
}

impl AnimalKind {
    fn speak(&self) -> &str {
        match self {
            AnimalKind::Dog(_) => "Woof!",
            AnimalKind::Cat(_) => "Meow!",
        }
    }

    fn name(&self) -> &str {
        match self {
            AnimalKind::Dog(a) | AnimalKind::Cat(a) => &a.name,
        }
    }
}
```

The tool understands that `speak` was pure and returns a static string, so it converts the return type to `&str`.  The common fields are factored out into `Animal`, and the polymorphic behaviour is expressed with an enum.

## 5. Configuration

The tool can be configured via a `migrate.toml` file in the project root:

```toml
[language]
source = "java"                # or "csharp"

[mapping]
"java.util.ArrayList" = "Vec"
"java.util.HashMap"   = "Map"

[policy]
use_actors_for_sync = true
use_dyn_dispatch    = false     # prefer match over enum when possible
generate_tests      = true      # also emit @test functions for original unit tests
```

## 6. Testing

`blaze‑migrate` includes a suite of test projects (mini‑applications with known structure) that are automatically migrated and then compiled with `blaze build`.  The resulting binaries must pass all original assertions (translated to `@test` functions).  The test suite includes:

- A single‑class application.
- A deep inheritance hierarchy with virtual methods.
- A class using interface‑based polymorphism.
- A class that manages a file resource with `try`/`finally`.
- A class that uses a generic collection.

All migrated projects must compile and pass their tests without manual intervention.
