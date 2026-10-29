//! The flat estimator: a homography from source pixel coordinates to candidate pixel coordinates.

use image::RgbImage;

use super::gray::Gray;
use super::linalg::{self, IDENTITY, Mat3};
use super::{LCN_EPS, Options, Quality, Refinement, Status};

/// A step moving no source corner by more than this at the level has settled.
const SETTLED_PX: f64 = 1e-3;
/// A refinement out of iterations whose last step was under this has settled enough: the
/// transform is within the tolerance ADR 0014 measured.
const SETTLED_ENOUGH_PX: f64 = 0.05;

/// The result of [`align_flat`].
#[derive(Debug, Clone, PartialEq)]
pub struct FlatAlignment {
    /// Row-major 3×3, source pixel coordinates (edge-based, see `gray`) to candidate pixel
    /// coordinates, normalised so the last entry is one. Identity-like when not registered.
    pub homography: Mat3,
    pub status: Status,
    pub quality: Quality,
    /// Gauss-Newton iterations over all levels.
    pub iterations: usize,
}

/// Coordinates normalised per image: `x_n = (x - w/2) / s`, `s = max(w, h) / 2`, so the same
/// normalised homography serves every pyramid level and the Gauss-Newton parameters are of one
/// scale.
#[derive(Debug, Clone, Copy)]
struct Norm {
    cx: f64,
    cy: f64,
    s: f64,
}

impl Norm {
    fn of(g: &Gray) -> Norm {
        Norm {
            cx: g.width as f64 / 2.0,
            cy: g.height as f64 / 2.0,
            s: g.width.max(g.height) as f64 / 2.0,
        }
    }
    /// Pixel to normalised.
    fn to(&self) -> Mat3 {
        [
            1.0 / self.s,
            0.0,
            -self.cx / self.s,
            0.0,
            1.0 / self.s,
            -self.cy / self.s,
            0.0,
            0.0,
            1.0,
        ]
    }
    /// Normalised to pixel.
    fn from(&self) -> Mat3 {
        [self.s, 0.0, self.cx, 0.0, self.s, self.cy, 0.0, 0.0, 1.0]
    }
}

/// A homography in normalised coordinates expressed in pixel coordinates of the given levels.
fn to_pixels(hn: &Mat3, source: &Norm, candidate: &Norm) -> Mat3 {
    linalg::normalise(&linalg::matmul(
        &candidate.from(),
        &linalg::matmul(hn, &source.to()),
    ))
}

fn to_normalised(hp: &Mat3, source: &Norm, candidate: &Norm) -> Mat3 {
    linalg::normalise(&linalg::matmul(
        &candidate.to(),
        &linalg::matmul(hp, &source.from()),
    ))
}

/// Estimates the homography that maps `source` onto `candidate`.
pub fn align_flat(source: &RgbImage, candidate: &RgbImage, opts: &Options) -> FlatAlignment {
    let gs = Gray::from_rgb(source);
    let gc = Gray::from_rgb(candidate);
    let scale_only = scale_between(&gs, &gc);
    let not = |status: Status, texture: f64| FlatAlignment {
        homography: scale_only,
        status,
        quality: Quality {
            overlap: 0.0,
            ncc: 0.0,
            inlier_fraction: 0.0,
            texture,
        },
        iterations: 0,
    };
    if gs.width < 8 || gs.height < 8 || gc.width < 8 || gc.height < 8 {
        return not(Status::LowTexture, 0.0);
    }
    let texture = texture_of(&gs).min(texture_of(&gc));
    if texture < opts.min_texture {
        return not(Status::LowTexture, texture);
    }
    // Bring the two images within a factor of about 1.4 of each other by halving the larger, so
    // every pyramid level pairs images of comparable pixel size and the local normalisation acts
    // at the same physical scale in both. The halvings are folded back into the result.
    let (gs, halved_s) = halve_towards(gs, &gc);
    let (gc, halved_c) = halve_towards(gc, &gs);
    let ps = pyramid(&gs, opts);
    let pc = pyramid(&gc, opts);
    let levels = ps.len().min(pc.len());
    if levels == 0 {
        return not(Status::NotRegistered, texture);
    }
    // Coarse search at the top level, in normalised coordinates.
    let top_s = &ps[levels - 1];
    let top_c = &pc[levels - 1];
    let mut hn = coarse_search(
        &top_s.lcn,
        &top_c.lcn,
        &Norm::of(&top_s.lcn),
        &Norm::of(&top_c.lcn),
    );
    let mut iterations = 0;
    let mut refinement = Refinement::Settled;
    let finest = 0;
    for l in (finest..levels).rev() {
        let (s, c) = (&ps[l], &pc[l]);
        let (ns, nc) = (Norm::of(&s.lcn), Norm::of(&c.lcn));
        let mut hp = to_pixels(&hn, &ns, &nc);
        let (outcome, iters) = gauss_newton(&s.lcn, &c.lcn, &c.gx, &c.gy, &mut hp, &ns, &nc, opts);
        iterations += iters;
        // A numerical failure ends the refinement; a level that merely ran out of iterations
        // hands its transform on to the finer levels, and the finest level's outcome stands.
        refinement = outcome;
        if outcome == Refinement::Failed {
            break;
        }
        hn = to_normalised(&hp, &ns, &nc);
    }
    let (s, c) = (&ps[finest], &pc[finest]);
    let (ns, nc) = (Norm::of(&s.lcn), Norm::of(&c.lcn));
    let hp_finest = to_pixels(&hn, &ns, &nc);
    let (overlap, ncc, inliers) = measure(&s.lcn, &c.lcn, &hp_finest, opts);
    let quality = Quality {
        overlap,
        ncc,
        inlier_fraction: inliers,
        texture,
    };
    let agrees = overlap >= opts.min_overlap && ncc >= opts.min_ncc && inliers >= opts.min_inliers;
    let registered = refinement == Refinement::Settled && agrees;
    let status = status_of(registered, refinement, agrees);
    let reduced = to_pixels(&hn, &Norm::of(&gs), &Norm::of(&gc));
    // Reduced source pixels are original pixels over 2^halved_s; the same for the candidate.
    let (fs, fc) = (f64::from(1u32 << halved_s), f64::from(1u32 << halved_c));
    let homography = linalg::normalise(&linalg::matmul(
        &[fc, 0.0, 0.0, 0.0, fc, 0.0, 0.0, 0.0, 1.0],
        &linalg::matmul(
            &reduced,
            &[1.0 / fs, 0.0, 0.0, 0.0, 1.0 / fs, 0.0, 0.0, 0.0, 1.0],
        ),
    ));
    FlatAlignment {
        homography: if registered { homography } else { scale_only },
        status,
        quality,
        iterations,
    }
}

/// Halves `g` while it is more than about 1.4 times the size of `other`; returns the result and
/// the number of halvings.
fn halve_towards(mut g: Gray, other: &Gray) -> (Gray, u32) {
    let mut halvings = 0;
    while (g.width as f64) >= 1.4 * other.width as f64 && g.width / 2 >= 16 && g.height / 2 >= 16 {
        g = g.downsample();
        halvings += 1;
    }
    (g, halvings)
}

/// The outcome from how the refinement ended and how the pair agrees under the best transform.
/// A numerical failure is never evidence about the pair; a refinement that ran out of iterations
/// on a pair that does not agree is the natural end for a pair that does not fit, and is reported
/// as not registered; one that ran out while the pair agrees is not converged.
pub(crate) fn status_of(registered: bool, refinement: Refinement, agrees: bool) -> Status {
    match (registered, refinement, agrees) {
        (true, _, _) => Status::Registered,
        (false, Refinement::Failed, _) => Status::NotConverged,
        (false, Refinement::Exhausted, true) => Status::NotConverged,
        (false, _, _) => Status::NotRegistered,
    }
}

/// The pure scaling between two image sizes: what a pair of the same scene at different
/// resolutions maps by before any estimate.
fn scale_between(s: &Gray, c: &Gray) -> Mat3 {
    let sx = c.width as f64 / s.width.max(1) as f64;
    let sy = c.height as f64 / s.height.max(1) as f64;
    [sx, 0.0, 0.0, 0.0, sy, 0.0, 0.0, 0.0, 1.0]
}

/// The texture score at a working size of about 256 pixels, so it means the same for every input
/// size.
fn texture_of(g: &Gray) -> f64 {
    let mut cur = g.clone();
    while cur.width.max(cur.height) > 384 && cur.width >= 16 && cur.height >= 16 {
        cur = cur.downsample();
    }
    cur.texture()
}

struct Level {
    lcn: Gray,
    gx: Gray,
    gy: Gray,
}

/// The normalised levels the estimator uses, finest first: levels above `max_side` on the longer
/// side are neither refined nor measured, so nothing is built for them.
fn pyramid(g: &Gray, opts: &Options) -> Vec<Level> {
    g.pyramid(opts.min_side)
        .into_iter()
        .filter(|raw| raw.width.max(raw.height) <= opts.max_side)
        .map(|raw| {
            // A light blur before normalising keeps interpolation error small against the
            // structure the residual is measured on.
            let lcn = raw.blur3().lcn(opts.lcn_radius, LCN_EPS);
            let (gx, gy) = lcn.gradients();
            Level { lcn, gx, gy }
        })
        .collect()
}

/// Zero-mean normalised cross-correlation of `s` against `c` warped by `hp`, over the source
/// pixels that land inside `c`, with the overlap fraction.
fn ncc_warped(s: &Gray, c: &Gray, hp: &Mat3) -> (f64, f64) {
    let (mut n, mut sa, mut sb, mut saa, mut sbb, mut sab) =
        (0usize, 0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let m = margin(s);
    for y in m..s.height.saturating_sub(m) {
        for x in m..s.width.saturating_sub(m) {
            let (px, py) = linalg::apply(hp, x as f64 + 0.5, y as f64 + 0.5);
            let Some(b) = sample_inside(c, px, py) else {
                continue;
            };
            let a = f64::from(s.get(x, y));
            let b = f64::from(b);
            n += 1;
            sa += a;
            sb += b;
            saa += a * a;
            sbb += b * b;
            sab += a * b;
        }
    }
    if n < 16 {
        return (-1.0, 0.0);
    }
    let nf = n as f64;
    let cov = sab / nf - (sa / nf) * (sb / nf);
    let va = saa / nf - (sa / nf).powi(2);
    let vb = sbb / nf - (sb / nf).powi(2);
    let ncc = if va > 1e-12 && vb > 1e-12 {
        cov / (va * vb).sqrt()
    } else {
        -1.0
    };
    let measured = (s.width.saturating_sub(2 * m) * s.height.saturating_sub(2 * m)).max(1);
    (ncc, nf / measured as f64)
}

/// The border left out of every measurement at this level: the fixed margin, but never more
/// than a fifth of the smaller side so tiny levels keep pixels to measure.
fn margin(g: &Gray) -> usize {
    (super::MARGIN as usize).min(g.width.min(g.height) / 5)
}

/// A sample of `c` at least its margin inside the image.
#[inline]
fn sample_inside(c: &Gray, px: f64, py: f64) -> Option<f32> {
    let m = margin(c) as f64;
    if px < m || py < m || px > c.width as f64 - m || py > c.height as f64 - m {
        return None;
    }
    c.sample(px, py)
}

/// The best similarity over a grid of rotations, scales and integer translations at the
/// coarsest level, by NCC with at least a third of the source overlapping.
fn coarse_search(s: &Gray, c: &Gray, ns: &Norm, nc: &Norm) -> Mat3 {
    let rotations: [f64; 9] = [-20.0, -15.0, -10.0, -5.0, 0.0, 5.0, 10.0, 15.0, 20.0];
    let scales: [f64; 5] = [0.8, 0.9, 1.0, 1.11, 1.25];
    let max_shift = (c.width.max(c.height) as i64) / 2;
    let mut best = (f64::NEG_INFINITY, IDENTITY);
    for &deg in &rotations {
        for &scale in &scales {
            let (sn, cs) = deg.to_radians().sin_cos();
            for dy in -max_shift..=max_shift {
                for dx in -max_shift..=max_shift {
                    // Normalised: rotate and scale about the source centre, then translate; the
                    // candidate centre is the origin of its normalised frame.
                    let (tx, ty) = (dx as f64 / nc.s, dy as f64 / nc.s);
                    let hn = [
                        scale * cs,
                        -scale * sn,
                        tx,
                        scale * sn,
                        scale * cs,
                        ty,
                        0.0,
                        0.0,
                        1.0,
                    ];
                    let hp = to_pixels(&hn, ns, nc);
                    let (ncc, overlap) = ncc_warped(s, c, &hp);
                    if overlap < 0.33 {
                        continue;
                    }
                    // Prefer the smaller motion among equal scores (stability on flat regions).
                    let score = ncc
                        - 1e-6 * (dx * dx + dy * dy) as f64
                        - 1e-4 * deg.abs()
                        - 1e-3 * (scale - 1.0f64).abs();
                    if score > best.0 {
                        best = (score, hn);
                    }
                }
            }
        }
    }
    best.1
}

/// Robust Gauss-Newton on the eight homography parameters in normalised coordinates, in place on
/// the pixel homography `hp` of this level. Returns whether it converged and the iterations used.
#[allow(clippy::too_many_arguments)]
fn gauss_newton(
    s: &Gray,
    c: &Gray,
    cgx: &Gray,
    cgy: &Gray,
    hp: &mut Mat3,
    ns: &Norm,
    nc: &Norm,
    opts: &Options,
) -> (Refinement, usize) {
    let n_px = s.width * s.height;
    let stride = ((n_px as f64 / 250_000.0).sqrt().ceil() as usize).max(1);
    let mut hn = to_normalised(hp, ns, nc);
    let huber = f64::from(opts.huber);
    let mut last_moved = f64::INFINITY;
    for iter in 0..opts.max_iterations {
        let mut a = vec![0.0f64; 64];
        let mut b = vec![0.0f64; 8];
        let mut used = 0usize;
        let hp_cur = to_pixels(&hn, ns, nc);
        let m = margin(s);
        let mut y = m + stride / 2;
        while y + m < s.height {
            let mut x = m + stride / 2;
            while x + m < s.width {
                let (xn, yn) = linalg::apply(&ns.to(), x as f64 + 0.5, y as f64 + 0.5);
                let d = hn[6] * xn + hn[7] * yn + hn[8];
                let (xpn, ypn) = (
                    (hn[0] * xn + hn[1] * yn + hn[2]) / d,
                    (hn[3] * xn + hn[4] * yn + hn[5]) / d,
                );
                let (px, py) = linalg::apply(&hp_cur, x as f64 + 0.5, y as f64 + 0.5);
                let (Some(cv), Some(gx), Some(gy)) = (
                    sample_inside(c, px, py),
                    cgx.sample(px, py),
                    cgy.sample(px, py),
                ) else {
                    x += stride;
                    continue;
                };
                let r = f64::from(cv) - f64::from(s.get(x, y));
                // Pixel gradients scale by the candidate's normalisation to normalised units.
                let (gx, gy) = (f64::from(gx) * nc.s, f64::from(gy) * nc.s);
                let j = [
                    gx * xn / d,
                    gx * yn / d,
                    gx / d,
                    gy * xn / d,
                    gy * yn / d,
                    gy / d,
                    -(gx * xpn + gy * ypn) * xn / d,
                    -(gx * xpn + gy * ypn) * yn / d,
                ];
                let w = if r.abs() <= huber {
                    1.0
                } else {
                    huber / r.abs()
                };
                for i in 0..8 {
                    b[i] += w * j[i] * r;
                    for k in 0..8 {
                        a[i * 8 + k] += w * j[i] * j[k];
                    }
                }
                used += 1;
                x += stride;
            }
            y += stride;
        }
        if used < 64 {
            return (Refinement::Failed, iter);
        }
        // Levenberg damping keeps a poorly determined direction from running away.
        for i in 0..8 {
            a[i * 8 + i] *= 1.0 + 1e-3;
            a[i * 8 + i] += 1e-9;
        }
        let Some(delta) = linalg::solve(a, b) else {
            return (Refinement::Failed, iter);
        };
        let mut next = hn;
        for (h, d) in next.iter_mut().zip(delta.iter()) {
            *h -= d;
        }
        hn = linalg::normalise(&next);
        if !hn.iter().all(|v| v.is_finite()) {
            return (Refinement::Failed, iter);
        }
        // Settled when no source corner moves more than a thousandth of a pixel at this level.
        let moved = delta_pixels(&delta, ns, nc);
        last_moved = moved;
        if moved < SETTLED_PX {
            *hp = to_pixels(&hn, ns, nc);
            return (Refinement::Settled, iter + 1);
        }
        if moved > nc.s * 4.0 {
            return (Refinement::Exhausted, iter + 1);
        }
    }
    // Out of iterations: settled enough when the last step was already small, otherwise the fit
    // never came to rest.
    *hp = to_pixels(&hn, ns, nc);
    if last_moved < SETTLED_ENOUGH_PX {
        (Refinement::Settled, opts.max_iterations)
    } else {
        (Refinement::Exhausted, opts.max_iterations)
    }
}

/// The largest pixel motion a parameter step implies at the source corners.
fn delta_pixels(delta: &[f64], ns: &Norm, nc: &Norm) -> f64 {
    let mut worst = 0.0f64;
    for (xn, yn) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let (xn, yn) = (xn * ns.cx / ns.s, yn * ns.cy / ns.s);
        let dxp = delta[0] * xn + delta[1] * yn + delta[2] - (delta[6] * xn + delta[7] * yn) * xn;
        let dyp = delta[3] * xn + delta[4] * yn + delta[5] - (delta[6] * xn + delta[7] * yn) * yn;
        worst = worst.max((dxp * dxp + dyp * dyp).sqrt() * nc.s);
    }
    worst
}

/// Overlap, NCC and inlier fraction of `s` against `c` warped by `hp`. Inliers are counted over
/// the structured pixels of the overlap: where the normalised source departs from its local mean
/// by at least the structure floor, so flat areas (which agree with anything) do not count.
fn measure(s: &Gray, c: &Gray, hp: &Mat3, opts: &Options) -> (f64, f64, f64) {
    let (ncc, overlap) = ncc_warped(s, c, hp);
    let (mut n, mut inl) = (0usize, 0usize);
    let thr = f64::from(opts.inlier_residual);
    let m = margin(s);
    for y in m..s.height.saturating_sub(m) {
        for x in m..s.width.saturating_sub(m) {
            let a = f64::from(s.get(x, y));
            if a.abs() < super::STRUCTURE_FLOOR {
                continue;
            }
            let (px, py) = linalg::apply(hp, x as f64 + 0.5, y as f64 + 0.5);
            let Some(b) = sample_inside(c, px, py) else {
                continue;
            };
            n += 1;
            if (f64::from(b) - a).abs() < thr {
                inl += 1;
            }
        }
    }
    (
        overlap,
        ncc,
        if n == 0 { 0.0 } else { inl as f64 / n as f64 },
    )
}
