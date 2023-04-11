pub mod additional_mass;
pub mod collider;
pub mod impulse;
pub mod rigid_body;
pub mod teleport;

pub use additional_mass::AdditionalMass;
pub use collider::ActiveCollisionTypes;
pub use collider::Collider;
pub use collider::SharedShape;
pub use impulse::Impulse;
pub use rigid_body::BodyType;
pub use rigid_body::RigidBody;
pub use teleport::Teleport;

#[derive(Debug, Clone, Default)]
pub struct AngularVelocity(pub glam::Vec3);

#[derive(Debug, Clone, Default)]
pub struct LinearVelocity(pub glam::Vec3);

// TODO: Move smaller components into this file
