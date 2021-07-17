use std::rc::Rc;

use cgmath::{Matrix4, Quaternion, Vector3};

#[derive(Debug, Clone)]
pub(crate) struct Node {
    parent: Option<Rc<Node>>,
    children: Vec<Rc<Node>>,
    translation: Vector3<f32>,
    scale: Vector3<f32>,
    rotation: Quaternion<f32>,
    skin_index: usize,
    matrix: Matrix4<f32>,
}
