use crossbeam::channel::Receiver;
use rapier3d::na::Matrix3x1;
use rapier3d::prelude::*;

pub const DEFAULT_COLLISION_GROUP: u32 = 0b01;
pub const PANEL_COLLISION_GROUP: u32 = 0b10;
pub const HAND_COLLISION_GROUP: u32 = 0b00000100;
pub const WALL_COLLISION_GROUP: u32 = 0b00001000;
pub const SENSOR_COLLISION_GROUP: u32 = 0b00010000;

/// TODO: This is *usually* 72fps on the Quest 2, but we may support higher resolutions later.
pub const DELTA_TIME: f32 = 1. / 72.;

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
        let integration_parameters = IntegrationParameters {
            dt: DELTA_TIME,
            ..Default::default()
        };

        let physics_pipeline = PhysicsPipeline::new();
        let impulse_joints = ImpulseJointSet::new();
        let multibody_joints = MultibodyJointSet::new();
        let ccd_solver = CCDSolver::new();

        PhysicsContext {
            physics_pipeline,
            gravity: [0., 0., 0.].into(),
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
}
