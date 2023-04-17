use inline_tweak::tweak;

use hotham::glam::Vec2;

pub fn thumbstick_influence(thumbstick: Vec2, target: Vec2) -> f32 {
    let d = thumbstick.dot(target);
    let r1 = tweak!(0.25);
    let r2 = tweak!(0.95);
    ((d - r1) / (r2 - r1)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hotham::glam::vec2;

    #[test]
    fn test_thumbstick_influence() {
        assert_eq!(thumbstick_influence(Vec2::ZERO, vec2(0.0, -1.0)), 0.0);
        assert_eq!(thumbstick_influence(vec2(0.0, 1.0), vec2(0.0, -1.0)), 0.0);
        assert_eq!(thumbstick_influence(vec2(0.0, -1.0), vec2(0.0, -1.0)), 1.0);
    }
}
