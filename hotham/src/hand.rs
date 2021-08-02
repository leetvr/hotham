use anyhow::Result;

pub(crate) struct Hand {}

impl Hand {
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub(crate) fn grip(&self, amount: f32) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use cgmath::Matrix4;

    use super::*;

    #[test]
    pub fn grip_test() {
        let hand = Hand::new();
        let before = get_joint_matrices(&hand);
        hand.grip(0.5).unwrap();
        let after = get_joint_matrices(&hand);
        assert_ne!(before, after);
    }

    fn get_joint_matrices(hand: &Hand) -> Vec<Matrix4<f32>> {
        Default::default()
    }
}
