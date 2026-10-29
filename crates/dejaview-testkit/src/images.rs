//! Synthetic images with known geometry for the alignment tests (PLAN task 4.0): flat scenes that
//! can be rendered under any homography and spherical scenes that can be rendered under any
//! rotation, both evaluated analytically so a transformed rendering carries no resampling error
//! and the ground truth is exact. Nothing here comes from the prototype (owner decision 11).

use image::{GrayImage, RgbImage};

/// SplitMix64: a small deterministic generator, enough for placing blobs. Not the statistics
/// generator (ADR 0016 pins that one); nothing here reaches a result.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }
}

/// One Gaussian blob of a flat scene, in scene coordinates (the source image spans `[0, 1]` in x
/// and `[0, aspect]` in y, where `aspect = height / width`).
#[derive(Debug, Clone)]
pub struct Blob {
    pub x: f64,
    pub y: f64,
    /// Standard deviation in scene units.
    pub sigma: f64,
    /// Signed amplitude per channel.
    pub rgb: [f64; 3],
}

/// A flat scene: a smooth base, blobs at many scales and a few gratings, so it has texture at
/// every frequency the pyramid sees. Rendered by evaluating the function at every pixel.
#[derive(Debug, Clone)]
pub struct Scene {
    pub blobs: Vec<Blob>,
    /// `(frequency, angle, amplitude, centre x, centre y, envelope sigma)` sine gratings in scene
    /// units, each under a Gaussian envelope so no texture repeats across the whole scene.
    pub gratings: Vec<(f64, f64, f64, f64, f64, f64)>,
    pub base: [f64; 3],
}

impl Scene {
    /// A scene with `blobs` blobs over a `width × height` canvas's aspect ratio, from a seed.
    pub fn random(seed: u64, blobs: usize, aspect: f64) -> Scene {
        let mut rng = Rng::new(seed);
        let base = [
            rng.range(0.35, 0.65),
            rng.range(0.35, 0.65),
            rng.range(0.35, 0.65),
        ];
        let blobs = (0..blobs)
            .map(|_| {
                // Sizes spread over two decades so every pyramid level has structure.
                let sigma = 0.004 * (10f64).powf(rng.range(0.0, 1.6));
                let amp = rng.range(0.25, 0.6) * if rng.unit() < 0.5 { -1.0 } else { 1.0 };
                Blob {
                    x: rng.range(-0.1, 1.1),
                    y: rng.range(-0.1, aspect + 0.1),
                    sigma,
                    rgb: [
                        amp * rng.range(0.5, 1.0),
                        amp * rng.range(0.5, 1.0),
                        amp * rng.range(0.5, 1.0),
                    ],
                }
            })
            .collect();
        let gratings = (0..4)
            .map(|_| {
                (
                    rng.range(3.0, 12.0),
                    rng.range(0.0, std::f64::consts::PI),
                    rng.range(0.04, 0.1),
                    rng.range(0.0, 1.0),
                    rng.range(0.0, aspect),
                    rng.range(0.08, 0.2),
                )
            })
            .collect();
        Scene {
            blobs,
            gratings,
            base,
        }
    }

    /// The scene's colour at a scene coordinate.
    pub fn at(&self, x: f64, y: f64) -> [f64; 3] {
        let mut c = self.base;
        for b in &self.blobs {
            let d2 = (x - b.x).powi(2) + (y - b.y).powi(2);
            let g = (-d2 / (2.0 * b.sigma * b.sigma)).exp();
            if g > 1e-6 {
                for (ch, amp) in c.iter_mut().zip(b.rgb.iter()) {
                    *ch += amp * g;
                }
            }
        }
        for &(f, a, amp, gx, gy, sigma) in &self.gratings {
            let env = (-((x - gx).powi(2) + (y - gy).powi(2)) / (2.0 * sigma * sigma)).exp();
            let v =
                amp * env * (2.0 * std::f64::consts::PI * f * (x * a.cos() + y * a.sin())).sin();
            for ch in &mut c {
                *ch += v;
            }
        }
        c
    }

    /// Renders the scene as the source image: pixel `(u, v)` samples the scene at its centre.
    pub fn render(&self, width: u32, height: u32) -> RgbImage {
        self.render_warped(width, height, &IDENTITY)
    }

    /// Renders the scene as seen through a homography `h` (row-major 3×3, pixel coordinates):
    /// pixel `(u, v)` of the output shows the scene point that the source pixel `h⁻¹(u, v)`
    /// would show, so `h` maps source pixel coordinates to output pixel coordinates exactly. The
    /// source is taken to be `width` pixels wide; see [`Scene::render_warped_from`] otherwise.
    pub fn render_warped(&self, width: u32, height: u32, h: &[f64; 9]) -> RgbImage {
        self.render_warped_from(width, width, height, h)
    }

    /// [`Scene::render_warped`] for an output of a different size from the source: the source
    /// image, whose pixel coordinates `h` takes, is `source_width` wide.
    pub fn render_warped_from(
        &self,
        source_width: u32,
        width: u32,
        height: u32,
        h: &[f64; 9],
    ) -> RgbImage {
        let inv = invert(h);
        let scale = f64::from(source_width);
        let mut out = RgbImage::new(width, height);
        for (u, v, p) in out.enumerate_pixels_mut() {
            let (px, py) = (f64::from(u) + 0.5, f64::from(v) + 0.5);
            let (sx, sy) = apply(&inv, px, py);
            let c = self.at(sx / scale, sy / scale);
            *p = image::Rgb(c.map(to_u8));
        }
        out
    }
}

/// A scene on the sphere: blobs around unit vectors, rendered to an equirectangular image under
/// any rotation exactly.
#[derive(Debug, Clone)]
pub struct SphereScene {
    /// `(centre, angular sigma in radians, rgb amplitude)`.
    pub blobs: Vec<([f64; 3], f64, [f64; 3])>,
    pub base: [f64; 3],
}

impl SphereScene {
    pub fn random(seed: u64, blobs: usize) -> SphereScene {
        let mut rng = Rng::new(seed);
        let base = [
            rng.range(0.35, 0.65),
            rng.range(0.35, 0.65),
            rng.range(0.35, 0.65),
        ];
        let blobs = (0..blobs)
            .map(|_| {
                // Uniform on the sphere.
                let z = rng.range(-1.0, 1.0);
                let lon = rng.range(-std::f64::consts::PI, std::f64::consts::PI);
                let r = (1.0 - z * z).sqrt();
                let centre = [r * lon.cos(), r * lon.sin(), z];
                let sigma = 0.01 * (10f64).powf(rng.range(0.0, 1.5));
                let amp = rng.range(0.25, 0.6) * if rng.unit() < 0.5 { -1.0 } else { 1.0 };
                (
                    centre,
                    sigma,
                    [
                        amp * rng.range(0.5, 1.0),
                        amp * rng.range(0.5, 1.0),
                        amp * rng.range(0.5, 1.0),
                    ],
                )
            })
            .collect();
        SphereScene { blobs, base }
    }

    /// The colour in direction `p` (a unit vector).
    pub fn at(&self, p: [f64; 3]) -> [f64; 3] {
        let mut c = self.base;
        for (centre, sigma, rgb) in &self.blobs {
            let dot = (p[0] * centre[0] + p[1] * centre[1] + p[2] * centre[2]).clamp(-1.0, 1.0);
            let angle = dot.acos();
            let g = (-(angle * angle) / (2.0 * sigma * sigma)).exp();
            if g > 1e-6 {
                for (ch, amp) in c.iter_mut().zip(rgb.iter()) {
                    *ch += amp * g;
                }
            }
        }
        c
    }

    /// Renders the scene as an equirectangular image of `width × height` (2:1) seen by a camera
    /// rotated by `r` (row-major 3×3): output pixel in direction `q` shows the scene at `rᵀ q`,
    /// so a point of the scene at `p` appears at direction `r p`, which is what an estimator
    /// recovering the rotation from the unrotated rendering to this one must return.
    pub fn render(&self, width: u32, height: u32, r: &[f64; 9]) -> RgbImage {
        let mut out = RgbImage::new(width, height);
        for (u, v, px) in out.enumerate_pixels_mut() {
            let q = direction(u, v, width, height);
            let p = [
                r[0] * q[0] + r[3] * q[1] + r[6] * q[2],
                r[1] * q[0] + r[4] * q[1] + r[7] * q[2],
                r[2] * q[0] + r[5] * q[1] + r[8] * q[2],
            ];
            *px = image::Rgb(self.at(p).map(to_u8));
        }
        out
    }
}

/// The unit direction of an equirectangular pixel's centre: longitude from `-π` at the left edge
/// to `π` at the right, latitude `π/2` at the top row.
pub fn direction(u: u32, v: u32, width: u32, height: u32) -> [f64; 3] {
    let lon =
        (f64::from(u) + 0.5) / f64::from(width) * 2.0 * std::f64::consts::PI - std::f64::consts::PI;
    let lat = std::f64::consts::FRAC_PI_2
        - (f64::from(v) + 0.5) / f64::from(height) * std::f64::consts::PI;
    [lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()]
}

/// A low-texture image: a gentle linear gradient with faint noise.
pub fn low_texture(width: u32, height: u32, seed: u64, noise: f64) -> RgbImage {
    let mut rng = Rng::new(seed);
    let mut out = RgbImage::new(width, height);
    for (u, v, p) in out.enumerate_pixels_mut() {
        let t = 0.4
            + 0.2 * (f64::from(u) / f64::from(width))
            + 0.1 * (f64::from(v) / f64::from(height));
        let n = rng.range(-noise, noise);
        *p = image::Rgb([to_u8(t + n), to_u8(t + n), to_u8(t + n)]);
    }
    out
}

/// Horizontal mirror of an image.
pub fn mirrored(img: &RgbImage) -> RgbImage {
    image::imageops::flip_horizontal(img)
}

/// Luminance as an 8-bit grey image.
pub fn grey(img: &RgbImage) -> GrayImage {
    image::imageops::grayscale(img)
}

fn to_u8(v: f64) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

pub const IDENTITY: [f64; 9] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];

/// Applies a row-major 3×3 homography to a point.
pub fn apply(h: &[f64; 9], x: f64, y: f64) -> (f64, f64) {
    let d = h[6] * x + h[7] * y + h[8];
    (
        (h[0] * x + h[1] * y + h[2]) / d,
        (h[3] * x + h[4] * y + h[5]) / d,
    )
}

/// The inverse of a 3×3 matrix (adjugate over determinant).
pub fn invert(m: &[f64; 9]) -> [f64; 9] {
    let det = m[0] * (m[4] * m[8] - m[5] * m[7]) - m[1] * (m[3] * m[8] - m[5] * m[6])
        + m[2] * (m[3] * m[7] - m[4] * m[6]);
    [
        (m[4] * m[8] - m[5] * m[7]) / det,
        (m[2] * m[7] - m[1] * m[8]) / det,
        (m[1] * m[5] - m[2] * m[4]) / det,
        (m[5] * m[6] - m[3] * m[8]) / det,
        (m[0] * m[8] - m[2] * m[6]) / det,
        (m[2] * m[3] - m[0] * m[5]) / det,
        (m[3] * m[7] - m[4] * m[6]) / det,
        (m[1] * m[6] - m[0] * m[7]) / det,
        (m[0] * m[4] - m[1] * m[3]) / det,
    ]
}

/// Row-major product `a × b`.
pub fn matmul(a: &[f64; 9], b: &[f64; 9]) -> [f64; 9] {
    let mut out = [0.0; 9];
    for i in 0..3 {
        for j in 0..3 {
            out[i * 3 + j] = (0..3).map(|k| a[i * 3 + k] * b[k * 3 + j]).sum();
        }
    }
    out
}

/// A similarity about the image centre in pixel coordinates: scale, rotation (degrees,
/// counter-clockwise in image coordinates with y down), then translation.
pub fn similarity(
    width: u32,
    height: u32,
    scale: f64,
    rotation_deg: f64,
    tx: f64,
    ty: f64,
) -> [f64; 9] {
    let (cx, cy) = (f64::from(width) / 2.0, f64::from(height) / 2.0);
    let (s, c) = rotation_deg.to_radians().sin_cos();
    let (a, b) = (scale * c, scale * s);
    // x' = a (x - cx) - b (y - cy) + cx + tx ; y' = b (x - cx) + a (y - cy) + cy + ty
    [
        a,
        -b,
        -a * cx + b * cy + cx + tx,
        b,
        a,
        -b * cx - a * cy + cy + ty,
        0.0,
        0.0,
        1.0,
    ]
}

/// A similarity plus a perspective term: the row-major homography whose bottom row is
/// `[gx, gy, 1]` in centred coordinates, so corners move by about `gx × width / 2` of the span.
pub fn perspective(width: u32, height: u32, base: &[f64; 9], gx: f64, gy: f64) -> [f64; 9] {
    let (cx, cy) = (f64::from(width) / 2.0, f64::from(height) / 2.0);
    let to_centre = [1.0, 0.0, -cx, 0.0, 1.0, -cy, 0.0, 0.0, 1.0];
    let from_centre = [1.0, 0.0, cx, 0.0, 1.0, cy, 0.0, 0.0, 1.0];
    let p = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, gx, gy, 1.0];
    matmul(base, &matmul(&from_centre, &matmul(&p, &to_centre)))
}

/// A rotation matrix (row-major) from yaw about z, then pitch about y, then roll about x, in
/// degrees: `R = Rz(yaw) · Ry(pitch) · Rx(roll)`.
pub fn rotation(yaw_deg: f64, pitch_deg: f64, roll_deg: f64) -> [f64; 9] {
    let (sz, cz) = yaw_deg.to_radians().sin_cos();
    let (sy, cy) = pitch_deg.to_radians().sin_cos();
    let (sx, cx) = roll_deg.to_radians().sin_cos();
    let rz = [cz, -sz, 0.0, sz, cz, 0.0, 0.0, 0.0, 1.0];
    let ry = [cy, 0.0, sy, 0.0, 1.0, 0.0, -sy, 0.0, cy];
    let rx = [1.0, 0.0, 0.0, 0.0, cx, -sx, 0.0, sx, cx];
    matmul(&rz, &matmul(&ry, &rx))
}

/// The angle in degrees between two rotations: the rotation angle of `a · bᵀ`.
pub fn rotation_angle_between(a: &[f64; 9], b: &[f64; 9]) -> f64 {
    let bt = [b[0], b[3], b[6], b[1], b[4], b[7], b[2], b[5], b[8]];
    let m = matmul(a, &bt);
    let trace = m[0] + m[4] + m[8];
    ((trace - 1.0) / 2.0).clamp(-1.0, 1.0).acos().to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_warped_rendering_moves_the_scene_as_the_homography_says() {
        let scene = Scene::random(7, 40, 0.75);
        let h = similarity(64, 48, 1.0, 0.0, 5.0, -3.0);
        let a = scene.render(64, 48);
        let b = scene.render_warped(64, 48, &h);
        // Pixel (20, 20) of the source lands at (25, 17) of the warped image.
        assert_eq!(a.get_pixel(20, 20), b.get_pixel(25, 17));
    }

    #[test]
    fn rotations_compose_and_measure() {
        let r = rotation(30.0, 10.0, -5.0);
        let i = matmul(&r, &invert(&r));
        for (k, v) in i.iter().enumerate() {
            let want = if k % 4 == 0 { 1.0 } else { 0.0 };
            assert!((v - want).abs() < 1e-12);
        }
        assert!((rotation_angle_between(&rotation(30.0, 0.0, 0.0), &IDENTITY) - 30.0).abs() < 1e-9);
    }
}
