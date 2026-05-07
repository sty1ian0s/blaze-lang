# Phase 4 – Ecosystem Crate: `blaze‑ecs`

> **Goal:** Specify the `blaze‑ecs` crate, which provides a high‑performance, data‑oriented Entity Component System (ECS) for building simulations, games, and data‑intensive applications.  It is built entirely on Blaze’s linear type system, region‑based memory management, SoA layout (`@layout(soa)`), sliding windows, and compile‑time metaprogramming.  The ECS is designed to store millions of entities with thousands of components, process them in parallel, and automatically optimise memory access patterns.

---

## 1. Core Concepts

An **Entity** is a lightweight identifier (a generational index).  A **Component** is a plain data struct (annotated with `@data` and optionally `@layout(soa)`) that stores one aspect of an entity’s state.  A **System** is a pure function that operates on all entities that possess a specific set of components, mutating them.  **Resources** are singletons (global state) that systems can read or write.

The ECS world owns all entities, components, and resources.  Systems are registered with the world and are executed sequentially or in parallel according to a dependency graph.  The world automatically manages SoA storage for each component type, ensuring optimal cache utilisation without any manual layout decisions.

---

## 2. Entity and Component

### 2.1 `Entity`

```
pub struct Entity {
    pub id: u64,        // packed index + generation
}
```

- `Entity` is `@copy`.  It is a lightweight handle; the actual generation checks are performed by the world’s internal slot maps.
- The crate provides `Entity::is_alive(&self, world: &World) -> bool` for manual checks (requires a world reference).

### 2.2 Component Trait

```
pub trait Component: 'static + Sized + Send + Sync {
    fn storage_layout() -> StorageLayout;
}
```

- Implemented automatically via `@derive(Component)` or by the compiler for all types that satisfy the bounds.
- `StorageLayout` is an enum describing whether the component is stored as SoA (`@layout(soa)`) or AoS (fallback for small, trivially copyable types).  The world uses this to select the optimal storage backend.

---

## 3. World

### 3.1 `World`

```
pub struct World {
    entities: SlotMap<Entity, EntityMeta>,
    storages: Map<TypeId, Box<dyn Storage>>,
    resources: Map<TypeId, Box<dyn Any>>,
}
```

- Linear; `Dispose` drops all entities, components, and resources.
- `World::new() -> World` creates an empty world.

### 3.2 Entity Management

```
impl World {
    pub fn spawn(&mut self, components: impl IntoComponentBundle) -> Entity;
    pub fn despawn(&mut self, entity: Entity) -> bool;
    pub fn contains(&self, entity: Entity) -> bool;
    pub fn entity_count(&self) -> usize;
}
```

- `spawn` takes a tuple of components (e.g., `(Position, Velocity, Health)`) and creates a new entity, initialising each component storage.
- `despawn` removes the entity and all its components, freeing memory.  Returns `false` if the entity did not exist.
- `contains` checks if the entity is still alive.

### 3.3 Component Access

```
impl World {
    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T>;
    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T>;
    pub fn query<Q: Query>(&self) -> QueryIter<Q>;
    pub fn query_mut<Q: Query>(&mut self) -> QueryIterMut<Q>;
}
```

- `get` and `get_mut` access a single entity’s component.
- `query` and `query_mut` return iterators over all entities matching a typed query (see Section 4).

### 3.4 Resources

```
impl World {
    pub fn insert_resource<R: 'static + Send + Sync>(&mut self, resource: R);
    pub fn remove_resource<R: 'static>(&mut self) -> Option<R>;
    pub fn get_resource<R: 'static>(&self) -> Option<&R>;
    pub fn get_resource_mut<R: 'static>(&mut self) -> Option<&mut R>;
}
```

- Resources are singletons accessible from systems without entity iteration.

---

## 4. Queries

Queries are the main mechanism for systems to access component data.  They are types that describe a set of components to fetch, including optional components and filtering.

### 4.1 `Query` Trait

```
pub trait Query {
    type Item;
    type Fetch;
    fn fetch(storages: &World) -> Option<Self::Fetch>;
}
```

### 4.2 Predefined Query Types

The crate provides a set of query types that can be combined with a macro or written manually:

- `&T` – require component `T`, accessed read‑only.
- `&mut T` – require component `T`, accessed mutably.
- `Option<&T>` – optional component `T`, read‑only.
- `Option<&mut T>` – optional component `T`, mutable.
- `With<T>` / `Without<T>` – filter entities that have or lack component `T`.

A query is a tuple of these elements.  Example query type:

```
type MyQuery = (&Position, &mut Velocity, Option<&Health>, Without<Dead>);
```

The crate provides a macro `query!(…)` to simplify type definition:

```
let iter = world.query::<query!(&Position, &mut Velocity)>();
```

### 4.3 Query Iterators

```
pub struct QueryIter<Q: Query> { /* … */ }
impl<Q: Query> Iterator for QueryIter<Q> { … }

pub struct QueryIterMut<Q: Query> { /* … */ }
impl<Q: Query> Iterator for QueryIterMut<Q> { … }
```

- Yield `Q::Item` (a tuple of references).  Mutable queries ensure there is no aliasing between components (the borrow checker and linear types ensure safety; the runtime also uses internal checks).
- The iterator automatically uses SoA‑aware sliding windows when the underlying storage is SoA, yielding `WindowedView` for each component, which the compiler auto‑vectorises.

---

## 5. Systems

### 5.1 `System` Trait

```
pub trait System {
    fn run(&mut self, world: &mut World);
    fn name(&self) -> &'static str;
    fn dependencies(&self) -> Vec<&'static str> { Vec::new() }
}
```

- The `dependencies` method returns the names of systems that must run before this one.  This allows the scheduler to parallelise systems.

### 5.2 Derived Systems

Systems can be implemented manually, but the crate provides a `#[system]` attribute on functions that satisfy the following signature:

```
fn my_system(query: Query!(&mut Position, &Velocity), res: Res<DeltaTime>) { … }
```

The attribute generates a struct that implements `System`, extracts the `World` into typed queries and resources, and calls the function.  The generated struct uses linear types to ensure the function consumes its arguments correctly.

### 5.3 `SystemStage`

```
pub struct SystemStage {
    systems: Vec<Box<dyn System>>,
    thread_pool: Option<ThreadPool>,
}

impl SystemStage {
    pub fn new() -> SystemStage;
    pub fn add_system(&mut self, system: impl System + 'static);
    pub fn run(&mut self, world: &mut World);
}
```

- A `SystemStage` holds a set of systems.  When executed, it schedules them according to their dependencies and runs them, parallelising independent systems using the thread pool.
- The `run` method automatically calls `world::sync()` after each system to respect side effects.

---

## 6. SoA‑Aware Storage

The world stores each component type in a dedicated storage container.  The container type is chosen based on the `StorageLayout`:

- `SoAStorage<T>` – for components annotated with `@layout(soa)`.  This storage separates each field into its own `Vec`, enabling cache‑friendly iteration.
- `AoSStorage<T>` – for small, trivially copyable components (≤16 bytes, no pointers).  Stores an array of structs (AoS) that is still iterated with auto‑vectorisation.
- `SparseStorage<T>` – for components that only exist on a subset of entities, backed by a `SlotMap`.

The storage provides methods `get(entity)`, `insert(entity, component)`, `remove(entity)`, `iter()` / `iter_mut()`, `slice()`, `window_soa()`.  The query iterators call these to build the final windowed iterators.

---

## 7. Integration with `blaze‑gui` and `blaze‑physics`

The ECS is designed to integrate with the rest of the Blaze ecosystem:

- `blaze‑physics2d` / `blaze‑physics3d` can define components (`RigidBody`, `Collider`) and systems (`integrate_physics`, `resolve_collisions`) that run on the ECS world.
- `blaze‑gui` can treat the GUI widget tree as an ECS world, where each widget is an entity and layout, style, and event handlers are components.
- The `blaze‑wasm` crate can serialise the entire ECS world (entities + components) into a binary format for save/load or network transfer.

---

## 8. Error Handling

```
pub enum EcsError {
    EntityNotFound,
    ComponentMissing,
    ResourceMissing,
    StorageMismatch,
}
```

- Most ECS operations are infallible (they assume correct setup).  `get` methods return `Option`.  `query` panics if the component storages are not found, which should never happen in a correctly configured world.

---

## 9. Implementation Notes

- The crate uses compile‑time reflection (`std::meta`) to automatically register component types when they are first used in `spawn` or `insert_resource`.  The `Component` trait’s `storage_layout` method is generated by `@derive(Component)`.
- The scheduler for system stages builds a directed acyclic graph (DAG) of systems from their declared dependencies.  It then uses a work‑stealing thread pool to run as many systems concurrently as possible.  This is entirely deterministic if systems are pure (which they should be, given they only affect the world through queries and resources).
- All query types are zero‑cost abstractions; the generated code is equivalent to hand‑written loops over SoA slices.  The compiler inlines all iterator adapter chains and applies auto‑vectorisation and auto‑parallelisation (if the system is pure, which is determined by its effect set).

---

## 10. Testing

- **Spawn and despawn:** Create a world, spawn an entity with components, retrieve them, despawn, verify they’re gone.
- **Queries:** Spawn multiple entities with different component combinations, run a query, verify the iterated set matches expectations.
- **Mutable queries:** Spawn entities, run a mutable query that modifies components, verify changes are reflected.
- **System scheduling:** Register three systems with a dependency chain (A depends on B, C independent), run them, verify execution order respects dependencies.
- **SoA iteration:** For a component with `@layout(soa)`, spawn many entities, run a query that multiplies fields, and verify the compiler‑generated code uses SoA sliding windows (test by checking memory access pattern or by benchmarking).
- **Resource access:** Insert a resource, access it from within a system, verify correct value.
- **Concurrency:** (Stress test) Create many entities and register a system that touches them all; run on multiple threads and verify no data races or deadlocks.

All tests must pass on all supported platforms.
