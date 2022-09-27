use anyhow::{anyhow, Result};
use crossbeam::channel::Receiver;
use hecs::{Entity, World};
use rapier3d::na::Matrix3x1;
use rapier3d::prelude::*;

use crate::components::{Collider as ColliderComponent, RigidBody as RigidBodyComponent};

pub const DEFAULT_COLLISION_GROUP: u32 = 0b01;
pub const PANEL_COLLISION_GROUP: u32 = 0b10;

pub struct PhysicsContext {
    pub physics_pipeline: PhysicsPipeline,
    pub gravity: Matrix3x1<f32>,
    pub query_pipeline: QueryPipeline,
    pub colliders: ColliderSet,
    pub broad_phase: BroadPhase,
    pub narrow_phase: NarrowPhase,
    pub rigid_bodies: RigidBodySet,
    pub island_manager: IslandManager,
    pub collision_recv: Receiver<CollisionEvent>,
    pub contact_force_recv: Receiver<ContactForceEvent>,
    pub event_handler: ChannelEventCollector,
    pub integration_parameters: IntegrationParameters,
    pub impulse_joints: ImpulseJointSet,
    pub multibody_joints: MultibodyJointSet,
    pub ccd_solver: CCDSolver,
}

impl Default for PhysicsContext {
    fn default() -> Self {
        let (collision_send, collision_recv) = crossbeam::channel::unbounded();
        let (contact_force_send, contact_force_recv) = crossbeam::channel::unbounded();
        let event_handler = ChannelEventCollector::new(collision_send, contact_force_send);
        let gravity: Matrix3x1<f32> = vector![0.0, 0.0, 0.0]; // TODO: no gravity in SPACE baby! But some games may uh, need this.
        let mut integration_parameters = IntegrationParameters::default();

        // TODO: This is *usually* 72fps on the Quest 2, but we may support higher resolutions later.
        integration_parameters.dt = 1. / 72.;
        let physics_pipeline = PhysicsPipeline::new();
        let impulse_joints = ImpulseJointSet::new();
        let multibody_joints = MultibodyJointSet::new();
        let ccd_solver = CCDSolver::new();

        PhysicsContext {
            physics_pipeline,
            gravity,
            query_pipeline: QueryPipeline::new(),
            colliders: ColliderSet::new(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            rigid_bodies: RigidBodySet::new(),
            island_manager: IslandManager::new(),
            collision_recv,
            contact_force_recv,
            event_handler,
            integration_parameters,
            impulse_joints,
            multibody_joints,
            ccd_solver,
        }
    }
}

impl PhysicsContext {
    pub fn update(&mut self) {
        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            &(),
            &self.event_handler,
        );

        self.query_pipeline
            .update(&self.island_manager, &self.rigid_bodies, &self.colliders);
    }

    pub fn create_rigid_body_and_collider(
        &mut self,
        entity: Entity,
        rigid_body: RigidBody,
        mut collider: Collider,
    ) -> (RigidBodyComponent, ColliderComponent) {
        collider.user_data = entity.to_bits().get() as _;
        let rigid_body_handle = self.rigid_bodies.insert(rigid_body);

        // TODO: Users may wish to pass in their own interaction groups.
        collider.set_collision_groups(InteractionGroups::new(
            DEFAULT_COLLISION_GROUP,
            DEFAULT_COLLISION_GROUP,
        ));

        let a_collider_handle =
            self.colliders
                .insert_with_parent(collider, rigid_body_handle, &mut self.rigid_bodies);

        let collider_component = ColliderComponent {
            collisions_this_frame: vec![],
            handle: a_collider_handle,
        };
        let rigid_body_component = RigidBodyComponent {
            handle: rigid_body_handle,
        };

        (rigid_body_component, collider_component)
    }

    pub fn get_rigid_body<'a>(
        &'a mut self,
        world: &World,
        entity: Entity,
    ) -> Result<&'a mut RigidBody> {
        let rigid_body_handle = world.get::<RigidBodyComponent>(entity)?.handle;
        self.rigid_bodies
            .get_mut(rigid_body_handle)
            .ok_or_else(|| anyhow!("Unable to get Rigid Body for handle!"))
    }

    pub fn get_collider<'a>(
        &'a mut self,
        world: &World,
        entity: Entity,
    ) -> Result<&'a mut Collider> {
        let collider_handle = world.get::<ColliderComponent>(entity)?.handle;
        self.colliders
            .get_mut(collider_handle)
            .ok_or_else(|| anyhow!("Unable to get Collider for handle!"))
    }
}
