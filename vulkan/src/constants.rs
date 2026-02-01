use math::Vec3;

#[allow(dead_code)]
pub const VK_VIEW_VOLUME_FAR: f32 = 1.0;
#[allow(dead_code)]
pub const VK_VIEW_VOLUME_NEAR: f32 = 0.0;
#[allow(dead_code)]
pub const VK_VIEW_VOLUME_RIGHT: f32 = 1.0;
#[allow(dead_code)]
pub const VK_VIEW_VOLUME_LEFT: f32 = -1.0;
#[allow(dead_code)]
pub const VK_VIEW_VOLUME_TOP: f32 = -1.0;
#[allow(dead_code)]
pub const VK_VIEW_VOLUME_BOTTOM: f32 = 1.0;

#[allow(dead_code)]
pub const VK_DIR_UP: Vec3<f32> = Vec3::new(0.0, 1.0, 0.0);
#[allow(dead_code)]
pub const VK_DIR_RIGHT: Vec3<f32> = Vec3::new(1.0, 0.0, 0.0);
#[allow(dead_code)]
pub const VK_DIR_FORWARDS: Vec3<f32> = Vec3::new(0.0, 0.0, -1.0);
