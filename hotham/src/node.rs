use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use cgmath::{Matrix4, Quaternion, Vector3};

use crate::mesh::Mesh;

#[derive(Debug, Clone)]
pub struct Node {
    pub parent: Option<Weak<Node>>,
    pub children: Vec<RefCell<Rc<Node>>>,
    pub translation: Vector3<f32>,
    pub scale: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub skin_index: usize,
    pub matrix: Matrix4<f32>,
    pub mesh: Option<Mesh>,
}
