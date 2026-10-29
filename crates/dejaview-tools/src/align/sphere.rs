//! The spherical estimator: the rotation taking a source equirectangular panorama's directions to
//! the candidate's. A scene point seen at direction `p` in the source appears at `R p` in the
//! candidate.

use image::RgbImage;

use super::flat::status_of;
use super::gray::Gray;
use super::linalg::{self, IDENTITY, Mat3};
use super::{LCN_EPS, Options, Quality, Refinement, Status};

/// The result of [`align_sphere`].
#[derive(Debug, Clone, PartialEq)]
pub struct SphereAlignment {
    /// Row-major rotation matrix: source direction to candidate direction.
    pub rotation: Mat3,
    /// `rotation` as yaw about z, pitch about y and roll about x, in degrees (`Rz · Ry · Rx`).
    pub yaw_deg: f64,
    pub pitch_deg: f64,
    pub roll_deg: f64,
    pub status: Status,
    pub quality: Quality,
    pub iterations: usize,
}

/// The unit direction of an equirectangular pixel's centre (longitude `-π` at the left edge,
/// latitude `π/2` at the top).
#[inline]
fn direction(x: f64, y: f64, width: f64, height: f64) -> [f64; 3] {
    let lon = x / width * 2.0 * std::f64::consts::PI - std::f64::consts::PI;
    let lat = std::f64::consts::FRAC_PI_2 - y / height * std::f64::consts::PI;
    [lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()]
}

/// The continuous pixel coordinate of a direction.
#[inline]
fn pixel(q: [f64; 3], width: f64, height: f64) -> (f64, f64) {
    let lon = q[1].atan2(q[0]);
    let lat = q[2].clamp(-1.0, 1.0).asin();
    (
        (lon + std::f64::consts::PI) / (2.0 * std::f64::consts::PI) * width,
        (std::f64::consts::FRAC_PI_2 - lat) / std::f64::consts::PI * height,
    )
}

/// Estimates the rotation that maps `source` onto `candidate`; both 2:1 equirectangular.
pub fn align_sphere(source: &RgbImage, candidate: &RgbImage, opts: &Options) -> SphereAlignment {
    let gs = Gray::from_rgb(source);
    let gc = Gray::from_rgb(candidate);
    let not = |status: Status, texture: f64| SphereAlignment {
        rotation: IDENTITY,
        yaw_deg: 0.0,
        pitch_deg: 0.0,
        roll_deg: 0.0,
        status,
        quality: Quality {
            overlap: 1.0,
            ncc: 0.0,
            inlier_fraction: 0.0,
            texture,
        },
        iterations: 0,
    };
    if gs.width < 16 || gs.height < 8 || gc.width < 16 || gc.height < 8 {
        return not(Status::LowTexture, 0.0);
    }
    let texture = texture_of(&gs).min(texture_of(&gc));
    if texture < opts.min_texture {
        return not(Status::LowTexture, texture);
    }
    let ps = pyramid(&gs, opts);
    let pc = pyramid(&gc, opts);
    let levels = ps.len().min(pc.len());
    if levels == 0 {
        return not(Status::NotRegistered, texture);
    }
    let mut r = coarse_search(&ps[levels - 1], &pc[levels - 1]);
    let mut iterations = 0;
    let mut refinement = Refinement::Settled;
    let finest = 0;
    for l in (finest..levels).rev() {
        let (outcome, iters) = gauss_newton(&ps[l], &pc[l], &mut r, opts);
        iterations += iters;
        refinement = outcome;
        if outcome == Refinement::Failed {
            break;
        }
    }
    let (ncc, inliers) = measure(&ps[finest], &pc[finest], &r, opts);
    let quality = Quality {
        overlap: 1.0,
        ncc,
        inlier_fraction: inliers,
        texture,
    };
    let agrees = ncc >= opts.min_ncc && inliers >= opts.min_inliers;
    let registered = refinement == Refinement::Settled && agrees;
    let status = status_of(registered, refinement, agrees);
    let rotation = if registered { r } else { IDENTITY };
    let (yaw_deg, pitch_deg, roll_deg) = linalg::yaw_pitch_roll(&rotation);
    SphereAlignment {
        rotation,
        yaw_deg,
        pitch_deg,
        roll_deg,
        status,
        quality: Quality {
            overlap: if registered { 1.0 } else { 0.0 },
            ..quality
        },
        iterations,
    }
}

fn texture_of(g: &Gray) -> f64 {
    let mut cur = g.clone();
    while cur.width > 512 && cur.height >= 16 {
        cur = cur.downsample();
    }
    cur.texture()
}

/// The normalised levels, finest first; levels wider than `max_side` are not built.
fn pyramid(g: &Gray, opts: &Options) -> Vec<Gray> {
    g.pyramid(opts.min_side)
        .into_iter()
        .filter(|raw| raw.width <= opts.max_side)
        .map(|raw| raw.blur3().lcn(opts.lcn_radius, LCN_EPS))
        .collect()
}

/// `c` warped into `s`'s pixel grid by `r`: value at each source pixel.
fn warp(s: &Gray, c: &Gray, r: &Mat3) -> Vec<f32> {
    let (w, h) = (s.width as f64, s.height as f64);
    let (cw, ch) = (c.width as f64, c.height as f64);
    let mut out = vec![0.0f32; s.width * s.height];
    for y in 0..s.height {
        for x in 0..s.width {
            let p = direction(x as f64 + 0.5, y as f64 + 0.5, w, h);
            let (px, py) = pixel(linalg::rotate(r, p), cw, ch);
            out[y * s.width + x] = c.sample_wrapped(px, py);
        }
    }
    out
}

/// Area weight of a source row: the cosine of its latitude.
fn row_weight(y: usize, height: usize) -> f64 {
    let lat = std::f64::consts::FRAC_PI_2 - (y as f64 + 0.5) / height as f64 * std::f64::consts::PI;
    lat.cos().max(0.0)
}

/// Weighted NCC between `s` and a warped candidate.
fn ncc_of(s: &Gray, warped: &[f32]) -> f64 {
    let (mut n, mut sa, mut sb, mut saa, mut sbb, mut sab) =
        (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for y in 0..s.height {
        let w = row_weight(y, s.height);
        for x in 0..s.width {
            let a = f64::from(s.get(x, y));
            let b = f64::from(warped[y * s.width + x]);
            n += w;
            sa += w * a;
            sb += w * b;
            saa += w * a * a;
            sbb += w * b * b;
            sab += w * a * b;
        }
    }
    if n <= 0.0 {
        return -1.0;
    }
    let cov = sab / n - (sa / n) * (sb / n);
    let va = saa / n - (sa / n).powi(2);
    let vb = sbb / n - (sb / n).powi(2);
    if va > 1e-12 && vb > 1e-12 {
        cov / (va * vb).sqrt()
    } else {
        -1.0
    }
}

/// The best rotation over every integer yaw shift and a small pitch and roll grid at the
/// coarsest level.
fn coarse_search(s: &Gray, c: &Gray) -> Mat3 {
    let mut best = (f64::NEG_INFINITY, IDENTITY);
    let step = 360.0 / s.width as f64;
    let tilt = [-10.0f64, -5.0, 0.0, 5.0, 10.0];
    for &pitch in &tilt {
        for &roll in &tilt {
            for i in 0..s.width {
                let yaw = i as f64 * step;
                let r = rotation_zyx(yaw, pitch, roll);
                let ncc = ncc_of(s, &warp(s, c, &r));
                let score = ncc - 1e-4 * (pitch.abs() + roll.abs());
                if score > best.0 {
                    best = (score, r);
                }
            }
        }
    }
    best.1
}

fn rotation_zyx(yaw_deg: f64, pitch_deg: f64, roll_deg: f64) -> Mat3 {
    let rz = linalg::exp_so3([0.0, 0.0, yaw_deg.to_radians()]);
    let ry = linalg::exp_so3([0.0, pitch_deg.to_radians(), 0.0]);
    let rx = linalg::exp_so3([roll_deg.to_radians(), 0.0, 0.0]);
    linalg::matmul(&rz, &linalg::matmul(&ry, &rx))
}

/// Robust Gauss-Newton on the three rotation parameters with a numerical Jacobian: six extra
/// warps per iteration, each a rotation step of half a pixel at this level.
fn gauss_newton(s: &Gray, c: &Gray, r: &mut Mat3, opts: &Options) -> (Refinement, usize) {
    let eps = std::f64::consts::PI / s.width as f64;
    let huber = f64::from(opts.huber);
    let mut last_moved = f64::INFINITY;
    for iter in 0..opts.max_iterations {
        let base = warp(s, c, r);
        let mut jac: Vec<Vec<f32>> = Vec::with_capacity(3);
        for k in 0..3 {
            let mut plus = [0.0; 3];
            plus[k] = eps;
            let mut minus = [0.0; 3];
            minus[k] = -eps;
            let wp = warp(s, c, &linalg::matmul(&linalg::exp_so3(plus), r));
            let wm = warp(s, c, &linalg::matmul(&linalg::exp_so3(minus), r));
            jac.push(
                wp.iter()
                    .zip(wm.iter())
                    .map(|(p, m)| (p - m) / (2.0 * eps as f32))
                    .collect(),
            );
        }
        let mut a = vec![0.0f64; 9];
        let mut b = vec![0.0f64; 3];
        for y in 0..s.height {
            let wrow = row_weight(y, s.height);
            for x in 0..s.width {
                let i = y * s.width + x;
                let res = f64::from(base[i]) - f64::from(s.get(x, y));
                let w = wrow
                    * if res.abs() <= huber {
                        1.0
                    } else {
                        huber / res.abs()
                    };
                let j = [
                    f64::from(jac[0][i]),
                    f64::from(jac[1][i]),
                    f64::from(jac[2][i]),
                ];
                for p in 0..3 {
                    b[p] += w * j[p] * res;
                    for q in 0..3 {
                        a[p * 3 + q] += w * j[p] * j[q];
                    }
                }
            }
        }
        for p in 0..3 {
            a[p * 3 + p] *= 1.0 + 1e-3;
            a[p * 3 + p] += 1e-12;
        }
        let Some(delta) = linalg::solve(a, b) else {
            return (Refinement::Failed, iter);
        };
        let step = [-delta[0], -delta[1], -delta[2]];
        let norm = (step[0] * step[0] + step[1] * step[1] + step[2] * step[2]).sqrt();
        if !norm.is_finite() || norm > 1.0 {
            return (Refinement::Exhausted, iter + 1);
        }
        *r = linalg::matmul(&linalg::exp_so3(step), r);
        // Settled when the step moves no direction by more than a thousandth of a pixel.
        let moved_px = norm * s.width as f64 / (2.0 * std::f64::consts::PI);
        last_moved = moved_px;
        if moved_px < 1e-3 {
            return (Refinement::Settled, iter + 1);
        }
    }
    // Out of iterations: settled enough when the last step was already small.
    if last_moved < 0.05 {
        (Refinement::Settled, opts.max_iterations)
    } else {
        (Refinement::Exhausted, opts.max_iterations)
    }
}

fn measure(s: &Gray, c: &Gray, r: &Mat3, opts: &Options) -> (f64, f64) {
    let warped = warp(s, c, r);
    let ncc = ncc_of(s, &warped);
    let thr = opts.inlier_residual;
    let (mut n, mut inl) = (0.0f64, 0.0f64);
    for y in 0..s.height {
        let w = row_weight(y, s.height);
        for x in 0..s.width {
            let a = s.get(x, y);
            if f64::from(a.abs()) < super::STRUCTURE_FLOOR {
                continue;
            }
            n += w;
            if (warped[y * s.width + x] - a).abs() < thr {
                inl += w;
            }
        }
    }
    (ncc, if n > 0.0 { inl / n } else { 0.0 })
}
