//! Mapping through an aligned frame (SPEC `aligned:<image>:<transform>`): the frame's coordinates
//! are its parent's, its pixels the image's. A location drawn on it goes back to the image's raw
//! frame through the inverse transform: a homography for flat images (boxes become the polygon of
//! their four corners), a rotation on the sphere for equirectangular ones (boxes become their
//! footprint, cut at the seam).

use super::sphere;
use super::{Location, LocationKind};
use crate::{Error, Result};

fn mat(params: &[f64]) -> Result<[f64; 9]> {
    params.try_into().map_err(|_| {
        Error::Submission(format!(
            "a 3×3 transform needs 9 parameters, got {}",
            params.len()
        ))
    })
}

fn invert(m: &[f64; 9]) -> Result<[f64; 9]> {
    let det = m[0] * (m[4] * m[8] - m[5] * m[7]) - m[1] * (m[3] * m[8] - m[5] * m[6])
        + m[2] * (m[3] * m[7] - m[4] * m[6]);
    if det.abs() < 1e-300 || !det.is_finite() {
        return Err(Error::Submission("the transform is singular".into()));
    }
    Ok([
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

fn apply(h: &[f64; 9], p: [f64; 2]) -> [f64; 2] {
    let d = h[6] * p[0] + h[7] * p[1] + h[8];
    [
        (h[0] * p[0] + h[1] * p[1] + h[2]) / d,
        (h[3] * p[0] + h[4] * p[1] + h[5]) / d,
    ]
}

/// The points a location is made of: a box's corners, a polygon's vertices, the unit square for
/// the whole image; `None` for `absent` and `view`.
fn points_of(loc: &Location) -> Result<Option<Vec<[f64; 2]>>> {
    Ok(match loc.kind {
        LocationKind::Bbox => {
            let b = loc
                .bbox
                .ok_or_else(|| Error::Submission("bbox location needs bbox".into()))?;
            Some(vec![[b[0], b[1]], [b[2], b[1]], [b[2], b[3]], [b[0], b[3]]])
        }
        LocationKind::WholeImage => Some(vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
        LocationKind::Polygon => Some(
            loc.polygon
                .clone()
                .ok_or_else(|| Error::Submission("polygon location needs polygon".into()))?,
        ),
        LocationKind::Absent => None,
        LocationKind::View => {
            return Err(Error::Submission(
                "a view location cannot sit on an aligned frame".into(),
            ));
        }
    })
}

/// Clamps mapped points to the unit square; an error when none lands inside.
fn clamp_inside(points: Vec<[f64; 2]>, target: &str) -> Result<(Vec<[f64; 2]>, bool)> {
    let inside = points
        .iter()
        .any(|v| (0.0..=1.0).contains(&v[0]) && (0.0..=1.0).contains(&v[1]));
    if !inside {
        return Err(Error::Submission(format!(
            "location maps wholly outside {target:?}"
        )));
    }
    let clamped: Vec<[f64; 2]> = points
        .iter()
        .map(|v| [v[0].clamp(0.0, 1.0), v[1].clamp(0.0, 1.0)])
        .collect();
    let clipped = clamped != points;
    Ok((clamped, clipped))
}

fn polygon_location(frame: &str, points: Vec<[f64; 2]>) -> Location {
    Location {
        frame: frame.to_string(),
        kind: LocationKind::Polygon,
        bbox: None,
        polygon: Some(points),
        yaw: None,
        pitch: None,
        fov: None,
    }
}

/// From an aligned frame (parent coordinates) to the image's raw frame through the inverse
/// homography `h` (image → parent, normalised).
pub fn map_through_homography(loc: &Location, h: &[f64], raw: &str) -> Result<(Location, bool)> {
    let inv = invert(&mat(h)?)?;
    let Some(points) = points_of(loc)? else {
        let mut out = loc.clone();
        out.frame = raw.to_string();
        return Ok((out, false));
    };
    let mapped: Vec<[f64; 2]> = points.iter().map(|p| apply(&inv, *p)).collect();
    let (clamped, clipped) = clamp_inside(mapped, raw)?;
    Ok((polygon_location(raw, clamped), clipped))
}

/// From the image's raw frame into an aligned frame through the homography `h`.
pub fn map_into_homography(loc: &Location, h: &[f64], frame: &str) -> Result<Location> {
    let h = mat(h)?;
    let Some(points) = points_of(loc)? else {
        let mut out = loc.clone();
        out.frame = frame.to_string();
        return Ok(out);
    };
    let mapped: Vec<[f64; 2]> = points.iter().map(|p| apply(&h, *p)).collect();
    let (clamped, _) = clamp_inside(mapped, frame)?;
    Ok(polygon_location(frame, clamped))
}

fn rotate(r: &[f64; 9], p: [f64; 3]) -> [f64; 3] {
    [
        r[0] * p[0] + r[1] * p[1] + r[2] * p[2],
        r[3] * p[0] + r[4] * p[1] + r[5] * p[2],
        r[6] * p[0] + r[7] * p[1] + r[8] * p[2],
    ]
}

fn transpose(r: &[f64; 9]) -> [f64; 9] {
    [r[0], r[3], r[6], r[1], r[4], r[7], r[2], r[5], r[8]]
}

/// The equirectangular points of a location's boundary on the sphere after rotating every
/// direction by `r`, as a footprint cut at the seam.
fn rotated_footprint(
    loc: &Location,
    r: &[f64; 9],
    target: &str,
) -> Result<Option<(Location, bool)>> {
    let Some(points) = points_of(loc)? else {
        return Ok(None);
    };
    // Boxes and the whole image are sampled along their edges, like a view's footprint.
    let samples: Vec<[f64; 2]> = match loc.kind {
        LocationKind::Bbox | LocationKind::WholeImage => {
            let b = loc.bbox.unwrap_or([0.0, 0.0, 1.0, 1.0]);
            sphere::boundary_of(&b, 16)
        }
        _ => points,
    };
    let dirs: Vec<[f64; 3]> = samples
        .iter()
        .map(|p| rotate(r, sphere::direction_of_equirect(p[0], p[1])))
        .collect();
    let centre = {
        let (mut sx, mut sy, mut sz) = (0.0, 0.0, 0.0);
        for d in &dirs {
            sx += d[0];
            sy += d[1];
            sz += d[2];
        }
        [sx, sy, sz]
    };
    let fp = sphere::footprint_of_directions(&dirs, centre);
    let mut out = loc.clone();
    out.frame = target.to_string();
    out.bbox = None;
    out.polygon = None;
    let clipped = match fp {
        sphere::Footprint::Polygon { vertices, clipped } => {
            out.kind = LocationKind::Polygon;
            out.polygon = Some(vertices);
            clipped
        }
        sphere::Footprint::Pole { y0, y1 } => {
            out.kind = LocationKind::Bbox;
            out.bbox = Some([0.0, y0, 1.0, y1]);
            false
        }
    };
    Ok(Some((out, clipped)))
}

/// From an aligned equirectangular frame to the image's raw frame through the inverse rotation.
pub fn map_through_rotation(loc: &Location, r: &[f64], raw: &str) -> Result<(Location, bool)> {
    let inv = transpose(&mat(r)?);
    match rotated_footprint(loc, &inv, raw)? {
        Some(m) => Ok(m),
        None => {
            let mut out = loc.clone();
            out.frame = raw.to_string();
            Ok((out, false))
        }
    }
}

/// From the image's raw frame into an aligned equirectangular frame through the rotation.
pub fn map_into_rotation(loc: &Location, r: &[f64], frame: &str) -> Result<Location> {
    let r = mat(r)?;
    match rotated_footprint(loc, &r, frame)? {
        Some((m, _)) => Ok(m),
        None => {
            let mut out = loc.clone();
            out.frame = frame.to_string();
            Ok(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bbox(frame: &str, b: [f64; 4]) -> Location {
        Location {
            frame: frame.into(),
            kind: LocationKind::Bbox,
            bbox: Some(b),
            polygon: None,
            yaw: None,
            pitch: None,
            fov: None,
        }
    }

    #[test]
    fn a_box_on_an_aligned_frame_goes_back_through_the_inverse_homography() {
        // The image sits 0.1 to the right in the parent: x_parent = x_image + 0.1.
        let h = [1.0, 0.0, 0.1, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let (m, clipped) = map_through_homography(
            &bbox("aligned:candidate:t1", [0.5, 0.5, 0.7, 0.7]),
            &h,
            "raw:candidate",
        )
        .unwrap();
        assert!(!clipped);
        assert_eq!(m.frame, "raw:candidate");
        let p = m.polygon.clone().unwrap();
        assert!(
            (p[0][0] - 0.4).abs() < 1e-12 && (p[2][0] - 0.6).abs() < 1e-12,
            "{p:?}"
        );
        // The same box forward again lands where it started.
        let back = map_into_homography(&m, &h, "aligned:candidate:t1").unwrap();
        let q = back.polygon.unwrap();
        assert!(
            (q[0][0] - 0.5).abs() < 1e-12 && (q[2][1] - 0.7).abs() < 1e-12,
            "{q:?}"
        );
        // A box past the image's edge is clipped; one wholly outside is refused.
        let (_, clipped) = map_through_homography(
            &bbox("aligned:candidate:t1", [0.0, 0.0, 0.2, 0.2]),
            &h,
            "raw:candidate",
        )
        .unwrap();
        assert!(clipped);
        assert!(
            map_through_homography(
                &bbox("aligned:candidate:t1", [0.0, 0.0, 0.05, 0.2]),
                &h,
                "raw:candidate"
            )
            .is_err()
        );
    }

    #[test]
    fn a_box_on_an_aligned_panorama_goes_back_through_the_inverse_rotation() {
        // A yaw of 90°: the image's directions turned a quarter to the left in the parent.
        let (s, c) = (90f64).to_radians().sin_cos();
        let r = [c, -s, 0.0, s, c, 0.0, 0.0, 0.0, 1.0];
        let (m, clipped) = map_through_rotation(
            &bbox("aligned:candidate:t1", [0.7, 0.45, 0.8, 0.55]),
            &r,
            "raw:candidate",
        )
        .unwrap();
        assert!(!clipped);
        assert_eq!(m.kind, LocationKind::Polygon);
        let xs: Vec<f64> = m.polygon.unwrap().iter().map(|p| p[0]).collect();
        let mean = xs.iter().sum::<f64>() / xs.len() as f64;
        // 0.75 in the parent is longitude 90°; a quarter back is 0° = x 0.5.
        assert!((mean - 0.5).abs() < 0.01, "{mean}");
    }
}
