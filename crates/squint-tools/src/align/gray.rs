//! Single-channel float images for the alignment estimators: luminance, pyramids, gradients,
//! local contrast normalisation and bilinear sampling. Continuous coordinates are edge-based: pixel
//! `(i, j)` covers `[i, i+1) × [j, j+1)` and its centre is `(i + 0.5, j + 0.5)`, so halving the
//! size halves every coordinate exactly.

use image::RgbImage;

#[derive(Debug, Clone)]
pub struct Gray {
    pub width: usize,
    pub height: usize,
    /// Row-major, `width × height`.
    pub data: Vec<f32>,
}

impl Gray {
    pub fn new(width: usize, height: usize) -> Gray {
        Gray {
            width,
            height,
            data: vec![0.0; width * height],
        }
    }

    /// Rec. 601 luminance in `[0, 1]`.
    pub fn from_rgb(img: &RgbImage) -> Gray {
        let (w, h) = (img.width() as usize, img.height() as usize);
        let data = img
            .pixels()
            .map(|p| {
                (0.299 * f32::from(p.0[0]) + 0.587 * f32::from(p.0[1]) + 0.114 * f32::from(p.0[2]))
                    / 255.0
            })
            .collect();
        Gray {
            width: w,
            height: h,
            data,
        }
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize) -> f32 {
        self.data[y * self.width + x]
    }

    /// Halves each dimension by averaging 2×2 blocks (odd rows and columns are dropped).
    pub fn downsample(&self) -> Gray {
        let (w, h) = (self.width / 2, self.height / 2);
        let mut out = Gray::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let s = self.get(2 * x, 2 * y)
                    + self.get(2 * x + 1, 2 * y)
                    + self.get(2 * x, 2 * y + 1)
                    + self.get(2 * x + 1, 2 * y + 1);
                out.data[y * w + x] = s * 0.25;
            }
        }
        out
    }

    /// A separable `[1 2 1] / 4` blur with clamped borders.
    pub fn blur3(&self) -> Gray {
        let (w, h) = (self.width, self.height);
        let mut tmp = Gray::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let l = self.get(x.saturating_sub(1), y);
                let r = self.get((x + 1).min(w - 1), y);
                tmp.data[y * w + x] = 0.25 * (l + 2.0 * self.get(x, y) + r);
            }
        }
        let mut out = Gray::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let u = tmp.get(x, y.saturating_sub(1));
                let d = tmp.get(x, (y + 1).min(h - 1));
                out.data[y * w + x] = 0.25 * (u + 2.0 * tmp.get(x, y) + d);
            }
        }
        out
    }

    /// Central-difference gradients, zero on the border.
    pub fn gradients(&self) -> (Gray, Gray) {
        let (w, h) = (self.width, self.height);
        let mut gx = Gray::new(w, h);
        let mut gy = Gray::new(w, h);
        for y in 1..h.saturating_sub(1) {
            for x in 1..w.saturating_sub(1) {
                gx.data[y * w + x] = 0.5 * (self.get(x + 1, y) - self.get(x - 1, y));
                gy.data[y * w + x] = 0.5 * (self.get(x, y + 1) - self.get(x, y - 1));
            }
        }
        (gx, gy)
    }

    /// Local contrast normalisation: `(I - mean) / sqrt(var + eps)` over a `(2r+1)²` window, so
    /// two renderings of the same structure under different tone or colour agree where the
    /// structure agrees. Computed with summed-area tables.
    pub fn lcn(&self, radius: usize, eps: f32) -> Gray {
        let (w, h) = (self.width, self.height);
        let sat = |f: &dyn Fn(f32) -> f64| -> Vec<f64> {
            let mut t = vec![0.0f64; (w + 1) * (h + 1)];
            for y in 0..h {
                let mut row = 0.0;
                for x in 0..w {
                    row += f(self.get(x, y));
                    t[(y + 1) * (w + 1) + x + 1] = t[y * (w + 1) + x + 1] + row;
                }
            }
            t
        };
        let sum = sat(&|v| f64::from(v));
        let sq = sat(&|v| f64::from(v) * f64::from(v));
        let window = |t: &[f64], x0: usize, y0: usize, x1: usize, y1: usize| -> f64 {
            t[y1 * (w + 1) + x1] - t[y0 * (w + 1) + x1] - t[y1 * (w + 1) + x0]
                + t[y0 * (w + 1) + x0]
        };
        let mut out = Gray::new(w, h);
        for y in 0..h {
            let (y0, y1) = (y.saturating_sub(radius), (y + radius + 1).min(h));
            for x in 0..w {
                let (x0, x1) = (x.saturating_sub(radius), (x + radius + 1).min(w));
                let n = ((x1 - x0) * (y1 - y0)) as f64;
                let mean = window(&sum, x0, y0, x1, y1) / n;
                let var = (window(&sq, x0, y0, x1, y1) / n - mean * mean).max(0.0);
                out.data[y * w + x] =
                    ((f64::from(self.get(x, y)) - mean) / (var + f64::from(eps)).sqrt()) as f32;
            }
        }
        out
    }

    /// Bilinear sample at a continuous coordinate; `None` outside the area covered by pixel
    /// centres.
    #[inline]
    pub fn sample(&self, x: f64, y: f64) -> Option<f32> {
        let (fx, fy) = (x - 0.5, y - 0.5);
        // Bounded in f64 before any cast: a non-finite or huge coordinate is simply outside.
        if !(fx >= 0.0
            && fy >= 0.0
            && fx <= (self.width - 1) as f64
            && fy <= (self.height - 1) as f64)
        {
            return None;
        }
        let (x0, y0) = (fx as usize, fy as usize);
        if x0 + 1 >= self.width || y0 + 1 >= self.height {
            // Allow the exact last centre; anything past it is outside.
            if (fx - (self.width - 1) as f64).abs() < 1e-9
                || (fy - (self.height - 1) as f64).abs() < 1e-9
            {
                let xi = x0.min(self.width - 1);
                let yi = y0.min(self.height - 1);
                return Some(self.get(xi, yi));
            }
            return None;
        }
        let (ax, ay) = ((fx - x0 as f64) as f32, (fy - y0 as f64) as f32);
        let top = self.get(x0, y0) * (1.0 - ax) + self.get(x0 + 1, y0) * ax;
        let bottom = self.get(x0, y0 + 1) * (1.0 - ax) + self.get(x0 + 1, y0 + 1) * ax;
        Some(top * (1.0 - ay) + bottom * ay)
    }

    /// Bilinear sample with horizontal wrap-around and vertical clamping, for equirectangular
    /// images whose left and right edges meet.
    #[inline]
    pub fn sample_wrapped(&self, x: f64, y: f64) -> f32 {
        let w = self.width as f64;
        let mut fx = (x - 0.5).rem_euclid(w);
        if !fx.is_finite() {
            fx = 0.0;
        }
        let fy = (y - 0.5).clamp(0.0, (self.height - 1) as f64);
        let x0 = (fx as usize).min(self.width - 1);
        let x1 = (x0 + 1) % self.width;
        let y0 = (fy as usize).min(self.height - 1);
        let y1 = (y0 + 1).min(self.height - 1);
        let (ax, ay) = ((fx - x0 as f64) as f32, (fy - y0 as f64) as f32);
        let top = self.get(x0, y0) * (1.0 - ax) + self.get(x1, y0) * ax;
        let bottom = self.get(x0, y1) * (1.0 - ax) + self.get(x1, y1) * ax;
        top * (1.0 - ay) + bottom * ay
    }

    /// Halving pyramid, finest first, until the smaller side would drop below `min_side`.
    pub fn pyramid(&self, min_side: usize) -> Vec<Gray> {
        let mut levels = vec![self.clone()];
        loop {
            let last = levels.last().map(|g| (g.width, g.height));
            match last {
                Some((w, h)) if w / 2 >= min_side && h / 2 >= min_side => {
                    let next = levels.last().map(Gray::downsample);
                    match next {
                        Some(n) => levels.push(n),
                        None => break,
                    }
                }
                _ => break,
            }
        }
        levels
    }

    /// A texture score: the square root of the smaller eigenvalue of the mean structure tensor
    /// of the blurred image, in intensity units per pixel. Zero for a flat or linearly graded
    /// image, where no translation is determined in at least one direction.
    pub fn texture(&self) -> f64 {
        let (gx, gy) = self.blur3().gradients();
        let n = (self.width * self.height) as f64;
        let (mut sxx, mut sxy, mut syy) = (0.0f64, 0.0f64, 0.0f64);
        for (a, b) in gx.data.iter().zip(gy.data.iter()) {
            let (a, b) = (f64::from(*a), f64::from(*b));
            sxx += a * a;
            sxy += a * b;
            syy += b * b;
        }
        let (sxx, sxy, syy) = (sxx / n, sxy / n, syy / n);
        let half = (sxx + syy) / 2.0;
        let d = (((sxx - syy) / 2.0).powi(2) + sxy * sxy).sqrt();
        (half - d).max(0.0).sqrt()
    }
}
