use bevy::math::Vec3;
pub fn box_intersects((a1, a2): (Vec3, Vec3), (b1, b2): (Vec3, Vec3)) -> bool {
    let a_min = Vec3::new(a1.x.min(a2.x), a1.y.min(a2.y), a1.z.min(a2.z));
    let a_max = Vec3::new(a1.x.max(a2.x), a1.y.max(a2.y), a1.z.max(a2.z));

    let b_min = Vec3::new(b1.x.min(b2.x), b1.y.min(b2.y), b1.z.min(b2.z));
    let b_max = Vec3::new(b1.x.max(b2.x), b1.y.max(b2.y), b1.z.max(b2.z));

    (a_min.x <= b_max.x && a_max.x >= b_min.x)
        && (a_min.y <= b_max.y && a_max.y >= b_min.y)
        && (a_min.z <= b_max.z && a_max.z >= b_min.z)
}
