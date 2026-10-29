//! Small dense linear algebra for the estimators: 3×3 homographies and rotations, and a Gaussian
//! elimination for the normal equations.

pub type Mat3 = [f64; 9];

pub const IDENTITY: Mat3 = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];

/// Row-major product.
pub fn matmul(a: &Mat3, b: &Mat3) -> Mat3 {
    let mut out = [0.0; 9];
    for i in 0..3 {
        for j in 0..3 {
            out[i * 3 + j] = (0..3).map(|k| a[i * 3 + k] * b[k * 3 + j]).sum();
        }
    }
    out
}

pub fn transpose(m: &Mat3) -> Mat3 {
    [m[0], m[3], m[6], m[1], m[4], m[7], m[2], m[5], m[8]]
}

/// Adjugate over determinant; `None` when singular.
pub fn invert(m: &Mat3) -> Option<Mat3> {
    let det = m[0] * (m[4] * m[8] - m[5] * m[7]) - m[1] * (m[3] * m[8] - m[5] * m[6])
        + m[2] * (m[3] * m[7] - m[4] * m[6]);
    if det.abs() < 1e-300 || !det.is_finite() {
        return None;
    }
    Some([
        (m[4] * m[8] - m[5] * m[7]) / det,
        (m[2] * m[7] - m[1] * m[8]) / det,
        (m[1] * m[5] - m[2] * m[4]) / det,
        (m[5] * m[6] - m[3] * m[8]) / det,
        (m[0] * m[8] - m[2] * m[6]) / det,
        (m[2] * m[3] - m[0] * m[5]) / det,
        (m[3] * m[7] - m[4] * m[6]) / det,
        (m[1] * m[6] - m[0] * m[7]) / det,
        (m[0] * m[4] - m[1] * m[3]) / det,
    ])
}

/// Applies a homography to a point.
#[inline]
pub fn apply(h: &Mat3, x: f64, y: f64) -> (f64, f64) {
    let d = h[6] * x + h[7] * y + h[8];
    (
        (h[0] * x + h[1] * y + h[2]) / d,
        (h[3] * x + h[4] * y + h[5]) / d,
    )
}

/// Scales a homography so its last entry is one.
pub fn normalise(h: &Mat3) -> Mat3 {
    if h[8].abs() < 1e-300 {
        return *h;
    }
    h.map(|v| v / h[8])
}

/// Matrix–vector product.
#[inline]
pub fn rotate(r: &Mat3, p: [f64; 3]) -> [f64; 3] {
    [
        r[0] * p[0] + r[1] * p[1] + r[2] * p[2],
        r[3] * p[0] + r[4] * p[1] + r[5] * p[2],
        r[6] * p[0] + r[7] * p[1] + r[8] * p[2],
    ]
}

/// The rotation matrix of a rotation vector (Rodrigues).
pub fn exp_so3(w: [f64; 3]) -> Mat3 {
    let theta = (w[0] * w[0] + w[1] * w[1] + w[2] * w[2]).sqrt();
    if theta < 1e-12 {
        return [1.0, -w[2], w[1], w[2], 1.0, -w[0], -w[1], w[0], 1.0];
    }
    let k = [w[0] / theta, w[1] / theta, w[2] / theta];
    let (s, c) = theta.sin_cos();
    let kx = [0.0, -k[2], k[1], k[2], 0.0, -k[0], -k[1], k[0], 0.0];
    let kx2 = matmul(&kx, &kx);
    let mut out = IDENTITY;
    for ((o, k1), k2) in out.iter_mut().zip(kx.iter()).zip(kx2.iter()) {
        *o += s * k1 + (1.0 - c) * k2;
    }
    out
}

/// The angle in degrees of the rotation taking `a` to `b`.
pub fn angle_between(a: &Mat3, b: &Mat3) -> f64 {
    let m = matmul(a, &transpose(b));
    let trace = m[0] + m[4] + m[8];
    ((trace - 1.0) / 2.0).clamp(-1.0, 1.0).acos().to_degrees()
}

/// Yaw about z, pitch about y and roll about x in degrees, for `R = Rz · Ry · Rx`.
pub fn yaw_pitch_roll(r: &Mat3) -> (f64, f64, f64) {
    let yaw = r[3].atan2(r[0]);
    let pitch = (-r[6]).clamp(-1.0, 1.0).asin();
    let roll = r[7].atan2(r[8]);
    (yaw.to_degrees(), pitch.to_degrees(), roll.to_degrees())
}

/// Solves `a x = b` for a dense `n × n` system by Gaussian elimination with partial pivoting.
/// `a` is row-major and consumed. `None` when singular.
pub fn solve(mut a: Vec<f64>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    for col in 0..n {
        let pivot = (col..n).max_by(|&i, &j| {
            a[i * n + col]
                .abs()
                .partial_cmp(&a[j * n + col].abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;
        if a[pivot * n + col].abs() < 1e-14 {
            return None;
        }
        if pivot != col {
            for k in 0..n {
                a.swap(pivot * n + k, col * n + k);
            }
            b.swap(pivot, col);
        }
        for row in col + 1..n {
            let f = a[row * n + col] / a[col * n + col];
            if f == 0.0 {
                continue;
            }
            for k in col..n {
                a[row * n + k] -= f * a[col * n + k];
            }
            b[row] -= f * b[col];
        }
    }
    let mut x = vec![0.0; n];
    for row in (0..n).rev() {
        let s: f64 = (row + 1..n).map(|k| a[row * n + k] * x[k]).sum();
        x[row] = (b[row] - s) / a[row * n + row];
    }
    if x.iter().all(|v| v.is_finite()) {
        Some(x)
    } else {
        None
    }
}
