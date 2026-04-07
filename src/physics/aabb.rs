use glam::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn from_center(center: Vec3, half_width: f32, half_height: f32) -> Self {
        Self {
            min: Vec3::new(center.x - half_width, center.y, center.z - half_width),
            max: Vec3::new(
                center.x + half_width,
                center.y + half_height * 2.0,
                center.z + half_width,
            ),
        }
    }

    pub fn offset(self, offset: Vec3) -> Self {
        Self {
            min: self.min + offset,
            max: self.max + offset,
        }
    }

    pub fn expand(self, delta: Vec3) -> Self {
        let mut min = self.min;
        let mut max = self.max;

        if delta.x < 0.0 {
            min.x += delta.x;
        } else {
            max.x += delta.x;
        }
        if delta.y < 0.0 {
            min.y += delta.y;
        } else {
            max.y += delta.y;
        }
        if delta.z < 0.0 {
            min.z += delta.z;
        } else {
            max.z += delta.z;
        }

        Self { min, max }
    }

    pub fn clip_x_collide(&self, other: &Aabb, dx: f32) -> f32 {
        self.clip_axis(other, dx, Axis::X)
    }

    pub fn clip_y_collide(&self, other: &Aabb, dy: f32) -> f32 {
        self.clip_axis(other, dy, Axis::Y)
    }

    pub fn clip_z_collide(&self, other: &Aabb, dz: f32) -> f32 {
        self.clip_axis(other, dz, Axis::Z)
    }

    /// Slab-method ray-AABB intersection. Returns the parametric t of the
    /// entry point along the ray `from → to`, or `None` if no hit.
    pub fn ray_clip(&self, from: Vec3, to: Vec3) -> Option<f32> {
        let dir = to - from;
        let mut t_min = 0.0f32;
        let mut t_max = 1.0f32;

        for i in 0..3 {
            let origin = [from.x, from.y, from.z][i];
            let d = [dir.x, dir.y, dir.z][i];
            let lo = [self.min.x, self.min.y, self.min.z][i];
            let hi = [self.max.x, self.max.y, self.max.z][i];

            if d.abs() < 1e-9 {
                if origin < lo || origin > hi {
                    return None;
                }
            } else {
                let inv = 1.0 / d;
                let mut t0 = (lo - origin) * inv;
                let mut t1 = (hi - origin) * inv;
                if t0 > t1 {
                    std::mem::swap(&mut t0, &mut t1);
                }
                t_min = t_min.max(t0);
                t_max = t_max.min(t1);
                if t_min > t_max {
                    return None;
                }
            }
        }

        Some(t_min)
    }

    fn clip_axis(&self, other: &Aabb, mut delta: f32, axis: Axis) -> f32 {
        let (c1, c2) = axis.cross_axes();

        if get(other.max, c1) <= get(self.min, c1) || get(other.min, c1) >= get(self.max, c1) {
            return delta;
        }
        if get(other.max, c2) <= get(self.min, c2) || get(other.min, c2) >= get(self.max, c2) {
            return delta;
        }

        if delta > 0.0 && get(other.max, axis) <= get(self.min, axis) {
            let clip = get(self.min, axis) - get(other.max, axis);
            if clip < delta {
                delta = clip;
            }
        } else if delta < 0.0 && get(other.min, axis) >= get(self.max, axis) {
            let clip = get(self.max, axis) - get(other.min, axis);
            if clip > delta {
                delta = clip;
            }
        }

        delta
    }
}

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    fn cross_axes(self) -> (Axis, Axis) {
        match self {
            Axis::X => (Axis::Y, Axis::Z),
            Axis::Y => (Axis::X, Axis::Z),
            Axis::Z => (Axis::X, Axis::Y),
        }
    }
}

fn get(v: Vec3, axis: Axis) -> f32 {
    match axis {
        Axis::X => v.x,
        Axis::Y => v.y,
        Axis::Z => v.z,
    }
}
