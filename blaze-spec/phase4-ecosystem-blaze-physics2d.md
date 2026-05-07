# Phase 4 – Ecosystem Crate: `blaze‑physics2d`

> **Goal:** Specify the `blaze‑physics2d` crate, which provides a 2D rigid‑body physics engine built on Blaze’s ECS, SoA layout, and sliding‑window iteration.  It is designed for real‑time simulations, games, and scientific visualisation, following Blaze’s data‑oriented philosophy to maximise throughput and determinism.  All simulation steps are pure (empty effect set) unless I/O or GPU effects are explicitly requested, enabling automatic parallelisation and vectorisation by the compiler.

---

## 1. Core Concepts

The crate models physical objects as **rigid bodies** that move and collide under forces and constraints.  It operates on large arrays of bodies stored in structure‑of‑arrays (SoA) form via `@layout(soa)` components, enabling cache‑efficient, vectorised iteration.  The simulation loop is split into discrete stages:

- **integration** – updates velocities and positions from forces and impulses.
- **broad‑phase collision detection** – identifies potential pairs of colliding bodies using a spatial hash or sweep‑and‑prune.
- **narrow‑phase collision detection** – computes contact points and penetration depths.
- **resolution** – applies impulses and penetration corrections to resolve collisions, and handles joints/constraints.
- **sleeping** – marks inactive bodies to skip processing.

All stages operate on slices of component data and use sliding windows or parallel iteration automatically provided by Blaze’s compiler.

---

## 2. Components

All components are defined with `@data` and, where appropriate, `@layout(soa)`.  They are stored in the ECS world (from `blaze‑ecs`) and queried by systems.

### 2.1 `RigidBody2D`

```
@data
@layout(soa)
pub struct RigidBody2D {
    pub pos: Vec2,
    pub vel: Vec2,
    pub angle: f32,
    pub angular_vel: f32,
    pub mass: f32,
    pub inv_mass: f32,
    pub inertia: f32,
    pub inv_inertia: f32,
    pub restitution: f32,
    pub friction: f32,
    pub body_type: BodyType,
    pub flags: BodyFlags,
}

pub enum BodyType { Static, Dynamic, Kinematic }
pub struct BodyFlags(u8);   // bit‑flags: is_sleeping, is_fixed_rotation, etc.
```

### 2.2 `Collider2D`

```
@data
@layout(soa)
pub struct Collider2D {
    pub shape: ColliderShape,
    pub local_pos: Vec2,       // offset from body center
    pub local_angle: f32,
    pub category_bits: u16,    // collision filtering
    pub mask_bits: u16,
}
```

- `ColliderShape` is an enum: `Circle { radius: f32 }`, `Box { half_extents: Vec2 }`, `Capsule { a: Vec2, b: Vec2, radius: f32 }`, `Polygon { vertices: Vec<Vec2> }`, `Edge { a: Vec2, b: Vec2 }`.  For SoA purposes, the shape is stored as a tagged union, but the ECS may partition by shape variant using `@variant_partition` (applied automatically by the system).

### 2.3 `Contact`

```
@data
@layout(soa)
pub struct Contact {
    pub point: Vec2,
    pub normal: Vec2,
    pub penetration: f32,
    pub tangent: Vec2,
    pub normal_impulse: f32,
    pub tangent_impulse: f32,
    pub body_a: Entity,
    pub body_b: Entity,
}
```

- Generated during narrow‑phase and consumed by the resolution stage.  Stored in a dedicated vector (not per‑entity).

---

## 3. Types and Constants

### 3.1 `Vec2`

```
@data
pub struct Vec2 { pub x: f32, pub y: f32 }
```

- Implements all standard operators (`+`, `-`, `*`, `/`, dot, cross, length, normalize, rotate, etc.) via `std::ops`.

### 3.2 `PhysicsConfig`

```
pub struct PhysicsConfig {
    pub gravity: Vec2,
    pub iterations: u32,           // solver iterations (8 typical)
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub allow_sleep: bool,
    pub sleep_threshold: f32,      // speed below which body can sleep
    pub time_to_sleep: f32,        // seconds below threshold before sleeping
    pub time_step: f32,            // fixed time step (1/60 typical)
    pub max_translation: f32,      // clamp to prevent explosion
    pub max_rotation: f32,
}
```

---

## 4. Systems

The crate provides a set of functions that implement each stage.  These are registered as systems in the ECS world.

### 4.1 `integrate_forces`

```
pub fn integrate_forces(
    mut bodies: Query!(&mut RigidBody2D),
    config: Res<PhysicsConfig>,
    dt: f32,
) {
    // applies gravity, damping, updates vel/pos via semi‑implicit Euler
}
```

- Pure function; auto‑parallelised over all bodies.

### 4.2 `broad_phase`

```
pub fn broad_phase(
    colliders: Query!(&Collider2D, &RigidBody2D),
    config: Res<PhysicsConfig>,
) -> Vec<(Entity, Entity)> {
    // builds a spatial‑hash grid or uses sweep‑and‑prune over AABBs
}
```

- Returns a list of potentially colliding entity pairs.  The grid is built using a pure parallel loop.

### 4.3 `narrow_phase`

```
pub fn narrow_phase(
    pairs: &[(Entity, Entity)],
    bodies: Query!(&RigidBody2D),
    colliders: Query!(&Collider2D),
    contacts: impl Insert<Contact>,
) {
    // performs GJK/EPA for convex shapes, stores contacts in a pre‑allocated array
}
```

- Pure function; no allocation inside the loop (contacts are pre‑allocated).  The collision detection kernels are written with sliding‑window loops over potential pairs.

### 4.4 `resolve_contacts`

```
pub fn resolve_contacts(
    contacts: &mut [Contact],
    mut bodies: Query!(&mut RigidBody2D),
    config: Res<PhysicsConfig>,
) {
    // sequential position and velocity resolution (iterations loop)
    // applies impulses, penetration correction, friction
}
```

- Not trivially parallelisable because contacts may share bodies; runs sequentially for correctness.

### 4.5 `sleep_manager`

```
pub fn sleep_manager(
    mut bodies: Query!(&mut RigidBody2D),
    config: Res<PhysicsConfig>,
    dt: f32,
) {
    // updates sleep timers, wakes bodies on contact, puts slow bodies to sleep
}
```

---

## 5. Integration with ECS

The crate exposes a `PhysicsPlugin` that, when added to a `SystemStage`, registers all required systems in the correct dependency order and inserts the required resources (`PhysicsConfig`, a contact pool).

```
pub fn add_physics_plugin(stage: &mut SystemStage, config: PhysicsConfig) {
    stage.insert_resource(config);
    stage.add_system(integrate_forces);
    stage.add_system(broad_phase);
    stage.add_system(narrow_phase);
    stage.add_system(resolve_contacts);
    stage.add_system(sleep_manager);
    // internal dependency metadata ensures correct ordering
}
```

---

## 6. Determinism

All systems are pure (effect `∅`) except `broad_phase` which may allocate a spatial grid (if dynamic), but the algorithm is deterministic given the same input.  The crate guarantees that identical initial conditions and config produce identical simulation output, which is critical for networking and replay.  The `--reproducible` flag of the compiler ensures that any floating‑point operations use fixed‑point rounding where needed.

---

## 7. Implementation Notes

- The broad‑phase uses a spatial hash grid with a fixed cell size.  The grid is built by iterating over all colliders and computing their AABBs (axis‑aligned bounding boxes).  The grid is stored as a `Map<(i32, i32), Vec<Entity>>` allocated from an arena per frame, then freed.
- The narrow‑phase uses the GJK (Gilbert‑Johnson‑Keerthi) distance algorithm for convex shapes, with EPA (Expanding Polytope Algorithm) for penetration depth.  Both are implemented as pure functions that take two convex shapes and return a `Contact` or `None`.
- All vector math is performed on SoA views: the compiler transforms `bodies.pos` into a `&[Vec2]` pointing to a `Vec2` stored as separate `x` and `y` arrays if `@layout(soa)` is applied to `Vec2` (which it is by the crate internally, not the user).  Actually, `Vec2` is small (8 bytes), so we may keep it AoS for simplicity, but the `RigidBody2D` itself is SoA, meaning all `pos.x` are contiguous, etc.  This provides outstanding cache locality.
- Contacts are stored in a ring buffer per‑frame to avoid per‑frame allocations.

---

## 8. Testing

- **Integration:** Set up a world with two bodies falling under gravity, step the simulation, verify positions change as expected.
- **Collision:** Place two boxes on a collision course; after several steps, verify that a contact is generated and they do not overlap.
- **Restitution:** Drop a ball with restitution 1.0; after several bounces, verify it returns to nearly the original height (within numeric tolerance).
- **Sleep:** Place a body below the sleep threshold; verify it eventually goes to sleep and stops being integrated.
- **Determinism:** Run two identical simulations with the same seed; compare binary output of all body states after 100 steps; must be identical.
- **Performance:** (Bench mark) Simulate 10,000 bodies with random initial velocities; measure frame time and verify it scales linearly with body count.

All tests must pass on all supported platforms.
