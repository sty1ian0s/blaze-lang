# Phase 4 – Ecosystem Crate: `blaze‑physics3d`

> **Goal:** Specify the `blaze‑physics3d` crate, which provides a 3D rigid‑body physics engine built on Blaze’s ECS, SoA layout, and sliding‑window iteration.  It is the natural extension of `blaze‑physics2d` into three dimensions, supporting the same data‑oriented design, determinism, and automatic parallelisation.  All simulation steps are pure unless explicitly noted, enabling maximum performance on modern multi‑core CPUs.

---

## 1. Core Concepts

The 3D physics engine mirrors the 2D version but uses 3D vectors, quaternions for orientation, and supports 3D collision shapes.  The simulation loop consists of identical stages: integration, broad‑phase, narrow‑phase, resolution, and sleeping.  Bodies and colliders are stored as SoA components in the ECS world.

---

## 2. Components

### 2.1 `RigidBody3D`

```
@data
@layout(soa)
pub struct RigidBody3D {
    pub pos: Vec3,
    pub vel: Vec3,
    pub orientation: Quat,
    pub angular_vel: Vec3,
    pub mass: f32,
    pub inv_mass: f32,
    pub inertia: Mat3,
    pub inv_inertia: Mat3,
    pub restitution: f32,
    pub friction: f32,
    pub body_type: BodyType,
    pub flags: BodyFlags,
}

pub enum BodyType { Static, Dynamic, Kinematic }
pub struct BodyFlags(u8);   // bit‑flags: is_sleeping, is_fixed_rotation, etc.
```

### 2.2 `Collider3D`

```
@data
@layout(soa)
pub struct Collider3D {
    pub shape: ColliderShape,
    pub local_pos: Vec3,
    pub local_orientation: Quat,
    pub category_bits: u16,
    pub mask_bits: u16,
}
```

- `ColliderShape` enum: `Sphere { radius: f32 }`, `Box { half_extents: Vec3 }`, `Capsule { a: Vec3, b: Vec3, radius: f32 }`, `Cylinder { half_height: f32, radius: f32 }`, `Cone { half_height: f32, radius: f32 }`, `Mesh { vertices: Vec<Vec3>, indices: Vec<u32> }`, `HeightField { heights: Vec<f32>, scale: Vec3 }`.  For SoA partitioning, convex shapes and mesh shapes are handled separately.

### 2.3 `Contact3D`

```
@data
@layout(soa)
pub struct Contact3D {
    pub point: Vec3,
    pub normal: Vec3,
    pub penetration: f32,
    pub tangent1: Vec3,
    pub tangent2: Vec3,
    pub normal_impulse: f32,
    pub tangent_impulse1: f32,
    pub tangent_impulse2: f32,
    pub body_a: Entity,
    pub body_b: Entity,
}
```

---

## 3. Types and Constants

### 3.1 `Vec3` and `Quat`

```
@data
pub struct Vec3 { pub x: f32, pub y: f32, pub z: f32 }
@data
pub struct Quat { pub x: f32, pub y: f32, pub z: f32, pub w: f32 }
pub struct Mat3 { /* … */ }   // 3x3 matrix
```

- `Vec3`, `Quat`, `Mat3` implement all standard operators, cross/dot products, rotations, quaternion exponentials, and matrix multiplication.  They are `@copy` (size ≤16 bytes, no pointers).

### 3.2 `PhysicsConfig3D`

```
pub struct PhysicsConfig3D {
    pub gravity: Vec3,
    pub iterations: u32,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub allow_sleep: bool,
    pub sleep_threshold: f32,
    pub time_to_sleep: f32,
    pub time_step: f32,
    pub max_translation: f32,
    pub max_rotation: f32,
}
```

---

## 4. Systems

The crate provides the same set of systems as `blaze‑physics2d`, adapted for 3D.

### 4.1 `integrate_forces_3d`

```
pub fn integrate_forces_3d(
    mut bodies: Query!(&mut RigidBody3D),
    config: Res<PhysicsConfig3D>,
    dt: f32,
) {
    // applies gravity, damping; updates vel/pos and angular vel/orientation via
    // semi‑implicit Euler; clamps values according to config limits.
}
```

### 4.2 `broad_phase_3d`

```
pub fn broad_phase_3d(
    colliders: Query!(&Collider3D, &RigidBody3D),
    config: Res<PhysicsConfig3D>,
) -> Vec<(Entity, Entity)> {
    // builds a 3D spatial‑hash grid or sweep‑and‑prune over AABBs in 3D.
}
```

### 4.3 `narrow_phase_3d`

```
pub fn narrow_phase_3d(
    pairs: &[(Entity, Entity)],
    bodies: Query!(&RigidBody3D),
    colliders: Query!(&Collider3D),
    contacts: impl Insert<Contact3D>,
) {
    // uses GJK/EPA in 3D for convex shapes; for mesh shapes, uses GJK against
    // individual triangles of the mesh (early‑out based on bounding boxes).
}
```

### 4.4 `resolve_contacts_3d`

```
pub fn resolve_contacts_3d(
    contacts: &mut [Contact3D],
    mut bodies: Query!(&mut RigidBody3D),
    config: Res<PhysicsConfig3D>,
) {
    // iterative position and velocity resolution in 3D (sequential loop).
}
```

### 4.5 `sleep_manager_3d`

```
// analogous to 2D sleep manager.
```

---

## 5. Integration with ECS

The crate exposes a `Physics3DPlugin` that registers all systems in the correct order, inserts the required resources, and sets up the contact pool.

```
pub fn add_physics3d_plugin(stage: &mut SystemStage, config: PhysicsConfig3D) {
    stage.insert_resource(config);
    stage.add_system(integrate_forces_3d);
    stage.add_system(broad_phase_3d);
    stage.add_system(narrow_phase_3d);
    stage.add_system(resolve_contacts_3d);
    stage.add_system(sleep_manager_3d);
}
```

---

## 6. Implementation Notes

- All algorithms are the 3D extensions of the 2D versions.  The GJK/EPA implementation handles 3D simplexes (tetrahedra) and polyhedral bounding volumes.
- For mesh colliders, the narrow‑phase first checks AABB overlap, then performs a coarse overlap test with the mesh’s internal broad‑phase structure (a BVH or k‑d tree built at mesh creation time, stored in the mesh component).  Only potentially overlapping triangles are tested.
- The broad‑phase spatial hash uses a fixed cell size (e.g., 2 meters), handling up to several million potential pairs efficiently.
- The SoA layout of `RigidBody3D` ensures that all `pos.x`, `pos.y`, `pos.z`, `vel.x`, etc. are contiguous arrays, enabling the compiler to auto‑vectorise integration and broad‑phase loops using SIMD instructions.

---

## 7. Testing

- **Integration:** Set up two spheres with gravity, step simulation, verify they accelerate and collide.
- **Collision:** Place a box and a sphere intersecting; verify a contact is generated and resolved, separating them.
- **Orientation:** Apply an angular velocity to a rigid body; verify its orientation evolves correctly.
- **Multiple shapes:** Register a dynamic body with a box collider and a static body with a mesh collider; simulate and verify no penetration.
- **Determinism:** Two identical 3D worlds must produce bit‑identical results after 100 steps.
- **Performance:** Simulate 1,000 spheres with random velocities and measure the frame time; should scale linearly.
