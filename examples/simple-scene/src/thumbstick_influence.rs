use inline_tweak::tweak;

use hotham::glam::{vec3, Vec2, Vec3};

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct InfluenceFactors {
    pub hand: Vec3, // hand neutral, hand forward, hand upward
    pub foot: Vec3, // foot neutral, foot forward, foot downward
}

impl InfluenceFactors {
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        InfluenceFactors {
            hand: self.hand.lerp(other.hand, t),
            foot: self.foot.lerp(other.foot, t),
        }
    }
}

const fn ifa(hand: Vec3, foot: Vec3) -> InfluenceFactors {
    InfluenceFactors { hand, foot }
}

// Influence is ordered as follows:
// hand neutral, hand forward, hand upward, foot neutral, foot forward, foot downward
const NEUTRAL_INFLUENCE: InfluenceFactors = ifa(vec3(1.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0));
const N: usize = 16;
const DIRECTION_INFLUENCE: [InfluenceFactors; N] = [
    ifa(vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0)),   // Kick
    ifa(vec3(0.0, 1.0, 0.0), vec3(0.0, 1.0, 0.0)),   // Kick-punch mix
    ifa(vec3(0.0, 1.0, 0.0), vec3(0.0, 1.0, 0.0)),   // Kick-punch mix
    ifa(vec3(0.0, 1.0, 0.0), vec3(1.0, 0.0, 0.0)),   // Punch
    ifa(vec3(0.0, 1.0, 0.0), vec3(1.0, 0.0, 0.0)),   // Punch
    ifa(vec3(0.0, 0.75, 0.25), vec3(1.0, 0.0, 0.0)), // Punch-elbow blend
    ifa(vec3(0.0, 0.25, 0.75), vec3(1.0, 0.0, 0.0)), // Elbow-punch blend
    ifa(vec3(0.0, 0.0, 1.0), vec3(1.0, 0.0, 0.0)),   // Elbow
    ifa(vec3(0.0, 0.0, 1.0), vec3(1.0, 0.0, 0.0)),   // Elbow
    ifa(vec3(0.0, 0.0, 1.0), vec3(0.0, 0.0, 1.0)),   // Elbow-knee mix
    ifa(vec3(0.0, 0.0, 1.0), vec3(0.0, 0.0, 1.0)),   // Elbow-knee mix
    ifa(vec3(1.0, 0.0, 0.0), vec3(0.0, 0.0, 1.0)),   // Knee
    ifa(vec3(1.0, 0.0, 0.0), vec3(0.0, 0.0, 1.0)),   // Knee
    ifa(vec3(1.0, 0.0, 0.0), vec3(0.0, 0.25, 0.75)), // Knee-kick blend
    ifa(vec3(1.0, 0.0, 0.0), vec3(0.0, 0.75, 0.25)), // Kick-knee blend
    ifa(vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0)),   // Kick
];

pub fn left_thumbstick_influence(thumbstick: Vec2) -> InfluenceFactors {
    thumbstick_influence(thumbstick.y.atan2(-thumbstick.x), thumbstick.length())
}

pub fn right_thumbstick_influence(thumbstick: Vec2) -> InfluenceFactors {
    thumbstick_influence(thumbstick.y.atan2(thumbstick.x), thumbstick.length())
}

fn thumbstick_influence(theta: f32, length: f32) -> InfluenceFactors {
    let floating_index = theta * (N as f32 / std::f32::consts::TAU) - 0.5 + N as f32;
    assert!(floating_index >= 0.0);
    let a = floating_index.floor() as usize % N;
    let b = (a + 1) % N;
    assert!(a < N);
    assert!(b < N);
    let angular_blend = floating_index.fract();
    let dir_influence = DIRECTION_INFLUENCE[a].lerp(&DIRECTION_INFLUENCE[b], angular_blend);

    let r1 = tweak!(0.25);
    let r2 = tweak!(0.95);
    let radial_blend = ((length - r1) / (r2 - r1)).clamp(0.0, 1.0);
    NEUTRAL_INFLUENCE.lerp(&dir_influence, radial_blend)
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_1_SQRT_2;

    use super::*;
    use hotham::glam::vec2;

    #[test]
    fn test_neutral_influence_sums() {
        let mut hand_sum = 0.0;
        let mut foot_sum = 0.0;
        for j in 0..3 {
            hand_sum += NEUTRAL_INFLUENCE.hand[j];
            foot_sum += NEUTRAL_INFLUENCE.foot[j];
        }
        assert_eq!(hand_sum, 1.0);
        assert_eq!(foot_sum, 1.0);
    }

    #[test]
    fn test_direction_influence_sums() {
        for i in 0..N {
            let mut hand_sum = 0.0;
            let mut foot_sum = 0.0;
            let InfluenceFactors { hand, foot } = &DIRECTION_INFLUENCE[i];
            for j in 0..3 {
                assert!(hand[j] >= 0.0, "i: {i}, hand: {}", hand);
                assert!(foot[j] >= 0.0, "i: {i}, foot: {}", foot);
                hand_sum += hand[j];
                foot_sum += foot[j];
            }
            assert_eq!(
                (hand_sum, foot_sum),
                (1.0, 1.0),
                "i: {i}, hand: {hand}, foot: {foot}"
            );
        }
    }

    #[test]
    fn test_thumbstick_influence() {
        assert_eq!(right_thumbstick_influence(Vec2::ZERO), NEUTRAL_INFLUENCE);
        assert_eq!(
            right_thumbstick_influence(vec2(0.0, 1.0)),
            ifa(vec3(0.0, 1.0, 0.0), vec3(1.0, 0.0, 0.0)),
        );
        assert_eq!(
            right_thumbstick_influence(vec2(1.0, 0.0)),
            ifa(vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0)),
        );
        assert_eq!(
            right_thumbstick_influence(vec2(0.0, -1.0)),
            ifa(vec3(1.0, 0.0, 0.0), vec3(0.0, 0.0, 1.0)),
        );
        assert_eq!(
            right_thumbstick_influence(vec2(-FRAC_1_SQRT_2, FRAC_1_SQRT_2)),
            ifa(vec3(0.0, 0.5, 0.5), vec3(1.0, 0.0, 0.0)), // Punch-elbow blend
        );
    }
}
