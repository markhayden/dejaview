//! Frames and locations. A frame is a coordinate space the model or a tool can point into:
//! `raw:<image>` for a canonical image, `aligned:<image>:<transform>` for a warped image,
//! `crop:<call>:<n>` for a tool's crop. Every location names a frame and maps deterministically
//! back to the raw frame of its root image, through bbox crops and, for `view` crops of a
//! panorama, the sphere geometry in `sphere`.

pub mod aligned;
pub mod sphere;

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::job::Projection;
use crate::{Error, Result};

/// A location as a tagged record with nullable fields (see SPEC "Frames and locations"). The
/// fields `kind` needs are set; every other field is null.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Location {
    /// A frame id as given in the conversation, for example `raw:candidate` or a crop a tool returned.
    pub frame: String,
    /// `bbox` and `polygon` take coordinates; `view` takes yaw, pitch and fov; `whole_image` and `absent` take nothing.
    pub kind: LocationKind,
    /// `[left, top, right, bottom]` as fractions of the frame's width and height, each between 0 and 1 (never pixels).
    pub bbox: Option<[f64; 4]>,
    /// Three to sixty-four `[x, y]` points as fractions of the frame's width and height, between 0 and 1.
    pub polygon: Option<Vec<[f64; 2]>>,
    /// Degrees, -180 to 180, for `view` on an equirectangular frame.
    pub yaw: Option<f64>,
    /// Degrees, -90 to 90, for `view`.
    pub pitch: Option<f64>,
    /// Degrees, 10 to 120, for `view`.
    pub fov: Option<f64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LocationKind {
    Bbox,
    Polygon,
    View,
    WholeImage,
    Absent,
}

/// How a derived frame sits inside its parent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Mapping {
    /// The frame covers this normalised box of the parent (may extend past the parent's edges).
    Bbox([f64; 4]),
    /// A square perspective view of an equirectangular parent (`sphere::View`).
    View { yaw: f64, pitch: f64, fov: f64 },
    /// An aligned frame of a flat image: the row-major 3×3 homography taking the image's
    /// normalised coordinates to the parent's. The frame shares the parent's grid; a location on
    /// it maps back to the image through the inverse.
    Homography(Vec<f64>),
    /// An aligned frame of an equirectangular image: the row-major rotation taking the image's
    /// directions to the parent's.
    Rotation(Vec<f64>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Frame {
    Raw {
        image: String,
        width: u32,
        height: u32,
        projection: Projection,
    },
    /// `image` warped into `parent`'s frame by a transform: the pixels are the image's, the
    /// coordinates the parent's, so the root is the image and locations map back through the
    /// inverse of `mapping` (a `Homography` or a `Rotation`).
    Aligned {
        image: String,
        transform_id: String,
        parent: String,
        mapping: Mapping,
        width: u32,
        height: u32,
        projection: Projection,
    },
    Crop {
        parent: String,
        mapping: Mapping,
        width: u32,
        height: u32,
        projection: Projection,
    },
}

impl Frame {
    pub fn projection(&self) -> Projection {
        match self {
            Frame::Raw { projection, .. }
            | Frame::Aligned { projection, .. }
            | Frame::Crop { projection, .. } => *projection,
        }
    }
}

/// What an aligned frame is made of, for [`Registry::add_aligned`].
#[derive(Debug, Clone)]
pub struct AlignedFrame<'a> {
    /// The image whose pixels the frame shows.
    pub image: &'a str,
    pub transform_id: &'a str,
    /// The frame whose grid it shares.
    pub parent: &'a str,
    /// `Mapping::Homography` or `Mapping::Rotation`, image to parent.
    pub mapping: Mapping,
    pub width: u32,
    pub height: u32,
}

/// The registry of frames for one run.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Registry {
    frames: BTreeMap<String, Frame>,
}

/// A location mapped to the raw frame of its root image.
#[derive(Debug, Clone, PartialEq)]
pub struct Mapped {
    pub location: Location,
    /// True when the mapping fell partly outside the parent and was clipped.
    pub partial: bool,
}

impl Mapped {
    pub fn bbox(&self) -> Option<[f64; 4]> {
        self.location.bbox
    }
}

impl std::ops::Deref for Mapped {
    type Target = Location;
    fn deref(&self) -> &Location {
        &self.location
    }
}

/// Helper to read a frame's root image.
pub trait FrameExt {
    fn root(&self) -> &str;
}

impl FrameExt for (&Registry, &Frame) {
    fn root(&self) -> &str {
        self.0.root_of_frame(self.1)
    }
}

impl Registry {
    pub fn new() -> Registry {
        Registry::default()
    }

    pub fn add_raw(&mut self, image: &str, width: u32, height: u32, projection: Projection) {
        self.frames.insert(
            format!("raw:{image}"),
            Frame::Raw {
                image: image.to_string(),
                width,
                height,
                projection,
            },
        );
    }

    pub fn add_crop(&mut self, id: &str, parent: &str, mapping: Mapping, width: u32, height: u32) {
        let projection = match &mapping {
            Mapping::Bbox(_) => self
                .frames
                .get(parent)
                .map(Frame::projection)
                .unwrap_or_default(),
            Mapping::View { .. } => Projection::Flat,
            // Not a crop mapping; the registry keeps the parent's projection and `to_root`
            // refuses the frame.
            Mapping::Homography(_) | Mapping::Rotation(_) => self
                .frames
                .get(parent)
                .map(Frame::projection)
                .unwrap_or_default(),
        };
        self.frames.insert(
            id.to_string(),
            Frame::Crop {
                parent: parent.to_string(),
                mapping,
                width,
                height,
                projection,
            },
        );
    }

    /// Registers an aligned frame (SPEC `aligned:<image>:<transform>`): `image` warped into
    /// `parent` by the transform; the frame takes the parent's projection.
    pub fn add_aligned(&mut self, id: &str, aligned: AlignedFrame<'_>) {
        let projection = self
            .frames
            .get(aligned.parent)
            .map(Frame::projection)
            .unwrap_or_default();
        self.frames.insert(
            id.to_string(),
            Frame::Aligned {
                image: aligned.image.to_string(),
                transform_id: aligned.transform_id.to_string(),
                parent: aligned.parent.to_string(),
                mapping: aligned.mapping,
                width: aligned.width,
                height: aligned.height,
                projection,
            },
        );
    }

    pub fn get(&self, id: &str) -> Option<FrameRef<'_>> {
        self.frames.get(id).map(|frame| FrameRef {
            registry: self,
            frame,
        })
    }

    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.frames.keys().map(String::as_str)
    }

    fn root_of_frame<'a>(&'a self, frame: &'a Frame) -> &'a str {
        let mut current = frame;
        loop {
            match current {
                Frame::Raw { image, .. } | Frame::Aligned { image, .. } => return image,
                Frame::Crop { parent, .. } => match self.frames.get(parent) {
                    Some(p) => current = p,
                    None => return parent,
                },
            }
        }
    }

    /// The validity table from the spec: required and forbidden fields per kind, geometry rules,
    /// frame existence and projection rules.
    pub fn validate(&self, loc: &Location) -> Result<()> {
        let frame = self
            .frames
            .get(&loc.frame)
            .ok_or_else(|| Error::Submission(format!("unknown frame {:?}", loc.frame)))?;
        let projection = frame.projection();
        let null = |name: &str, present: bool| -> Result<()> {
            if present {
                Err(Error::Submission(format!(
                    "{:?} location must not set {name}",
                    loc.kind
                )))
            } else {
                Ok(())
            }
        };
        let no_view = || -> Result<()> {
            null("yaw", loc.yaw.is_some())?;
            null("pitch", loc.pitch.is_some())?;
            null("fov", loc.fov.is_some())
        };
        match loc.kind {
            LocationKind::Bbox => {
                let b = loc
                    .bbox
                    .ok_or_else(|| Error::Submission("bbox location needs bbox".into()))?;
                null("polygon", loc.polygon.is_some())?;
                no_view()?;
                validate_bbox(&b)?;
                if projection == Projection::Equirectangular && b[0] <= 0.0 && b[2] >= 1.0 {
                    return Err(Error::Submission(
                        "bbox straddles the seam of an equirectangular frame".into(),
                    ));
                }
            }
            LocationKind::Polygon => {
                let p = loc
                    .polygon
                    .as_ref()
                    .ok_or_else(|| Error::Submission("polygon location needs polygon".into()))?;
                null("bbox", loc.bbox.is_some())?;
                no_view()?;
                validate_polygon(p)?;
                if projection == Projection::Equirectangular {
                    let min_x = p.iter().map(|v| v[0]).fold(f64::INFINITY, f64::min);
                    let max_x = p.iter().map(|v| v[0]).fold(f64::NEG_INFINITY, f64::max);
                    if min_x <= 0.0 && max_x >= 1.0 {
                        return Err(Error::Submission(
                            "polygon crosses the seam of an equirectangular frame".into(),
                        ));
                    }
                }
            }
            LocationKind::View => {
                null("bbox", loc.bbox.is_some())?;
                null("polygon", loc.polygon.is_some())?;
                if projection != Projection::Equirectangular {
                    return Err(Error::Submission(
                        "view locations need an equirectangular frame".into(),
                    ));
                }
                let yaw = loc
                    .yaw
                    .ok_or_else(|| Error::Submission("view location needs yaw".into()))?;
                let pitch = loc
                    .pitch
                    .ok_or_else(|| Error::Submission("view location needs pitch".into()))?;
                let fov = loc
                    .fov
                    .ok_or_else(|| Error::Submission("view location needs fov".into()))?;
                if !(-180.0..180.0).contains(&yaw) {
                    return Err(Error::Submission(format!("yaw {yaw} outside [-180, 180)")));
                }
                if !(-90.0..=90.0).contains(&pitch) {
                    return Err(Error::Submission(format!(
                        "pitch {pitch} outside [-90, 90]"
                    )));
                }
                if !(10.0..=120.0).contains(&fov) {
                    return Err(Error::Submission(format!("fov {fov} outside [10, 120]")));
                }
            }
            LocationKind::WholeImage | LocationKind::Absent => {
                null("bbox", loc.bbox.is_some())?;
                null("polygon", loc.polygon.is_some())?;
                no_view()?;
            }
        }
        Ok(())
    }

    /// Maps a location to the raw frame of its root image, clipping and flagging partial overlap.
    pub fn to_root(&self, loc: &Location) -> Result<Mapped> {
        self.validate(loc)?;
        let mut current = loc.clone();
        let mut partial = false;
        loop {
            let frame = self
                .frames
                .get(&current.frame)
                .ok_or_else(|| Error::Submission(format!("unknown frame {:?}", current.frame)))?;
            match frame {
                Frame::Raw { .. } => {
                    return Ok(Mapped {
                        location: current,
                        partial,
                    });
                }
                Frame::Aligned { image, mapping, .. } => {
                    // The frame's coordinates are the parent's; the pixels are the image's, so
                    // the location goes back through the inverse transform to the image's raw
                    // frame.
                    let raw = format!("raw:{image}");
                    let (next, clipped) = match mapping {
                        Mapping::Homography(h) => {
                            aligned::map_through_homography(&current, h, &raw)?
                        }
                        Mapping::Rotation(r) => aligned::map_through_rotation(&current, r, &raw)?,
                        Mapping::Bbox(_) | Mapping::View { .. } => {
                            return Err(Error::Submission(format!(
                                "aligned frame {:?} carries a crop mapping",
                                current.frame
                            )));
                        }
                    };
                    partial |= clipped;
                    current = next;
                }
                Frame::Crop {
                    parent, mapping, ..
                } => {
                    let (next, clipped) = match mapping {
                        Mapping::Homography(_) | Mapping::Rotation(_) => {
                            return Err(Error::Submission(format!(
                                "crop frame {:?} carries a transform mapping",
                                current.frame
                            )));
                        }
                        Mapping::Bbox(m) => map_through_bbox(&current, m, parent)?,
                        Mapping::View { yaw, pitch, fov } => sphere::map_through_view(
                            &current,
                            &sphere::View {
                                yaw: *yaw,
                                pitch: *pitch,
                                fov: *fov,
                            },
                            parent,
                        )?,
                    };
                    partial |= clipped;
                    current = next;
                }
            }
        }
    }

    /// Maps a root-frame location into a derived frame (the inverse of `to_root` for one hop).
    pub fn from_root(&self, frame_id: &str, root_loc: &Location) -> Result<Location> {
        let frame = self
            .frames
            .get(frame_id)
            .ok_or_else(|| Error::Submission(format!("unknown frame {frame_id:?}")))?;
        let mapping = match frame {
            Frame::Crop { mapping, .. } | Frame::Aligned { mapping, .. } => mapping,
            Frame::Raw { .. } => {
                return Err(Error::Submission(format!("{frame_id:?} is a raw frame")));
            }
        };
        let m = match mapping {
            Mapping::Bbox(m) => m,
            Mapping::Homography(h) => return aligned::map_into_homography(root_loc, h, frame_id),
            Mapping::Rotation(r) => return aligned::map_into_rotation(root_loc, r, frame_id),
            Mapping::View { yaw, pitch, fov } => {
                return sphere::map_into_view(
                    root_loc,
                    &sphere::View {
                        yaw: *yaw,
                        pitch: *pitch,
                        fov: *fov,
                    },
                    frame_id,
                );
            }
        };
        let w = m[2] - m[0];
        let h = m[3] - m[1];
        let inv = |x: f64, y: f64| [(x - m[0]) / w, (y - m[1]) / h];
        let mut out = root_loc.clone();
        out.frame = frame_id.to_string();
        if let Some(b) = root_loc.bbox {
            let [x0, y0] = inv(b[0], b[1]);
            let [x1, y1] = inv(b[2], b[3]);
            out.bbox = Some([x0, y0, x1, y1]);
        }
        if let Some(p) = &root_loc.polygon {
            out.polygon = Some(p.iter().map(|v| inv(v[0], v[1])).collect());
        }
        Ok(out)
    }
}

/// A frame plus its registry, so `root()` can follow parents.
#[derive(Debug, Clone, Copy)]
pub struct FrameRef<'a> {
    registry: &'a Registry,
    frame: &'a Frame,
}

impl FrameRef<'_> {
    pub fn root(&self) -> &str {
        self.registry.root_of_frame(self.frame)
    }
    pub fn projection(&self) -> Projection {
        self.frame.projection()
    }
    pub fn frame(&self) -> &Frame {
        self.frame
    }
}

impl std::ops::Deref for FrameRef<'_> {
    type Target = Frame;
    fn deref(&self) -> &Frame {
        self.frame
    }
}

fn validate_bbox(b: &[f64; 4]) -> Result<()> {
    if b.iter().any(|v| !v.is_finite() || !(0.0..=1.0).contains(v)) {
        return Err(Error::Submission(format!("bbox {b:?} outside [0, 1]")));
    }
    if !(b[0] < b[2] && b[1] < b[3]) {
        return Err(Error::Submission(format!(
            "bbox {b:?} needs x0 < x1 and y0 < y1"
        )));
    }
    Ok(())
}

fn validate_polygon(p: &[[f64; 2]]) -> Result<()> {
    if p.len() < 3 || p.len() > 64 {
        return Err(Error::Submission(format!(
            "polygon needs 3 to 64 vertices, has {}",
            p.len()
        )));
    }
    if p.iter()
        .flatten()
        .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
    {
        return Err(Error::Submission("polygon vertex outside [0, 1]".into()));
    }
    // Simple polygon: no two non-adjacent edges intersect.
    let n = p.len();
    for i in 0..n {
        for j in i + 1..n {
            let adjacent = j == i + 1 || (i == 0 && j == n - 1);
            if adjacent {
                continue;
            }
            if segments_intersect(p[i], p[(i + 1) % n], p[j], p[(j + 1) % n]) {
                return Err(Error::Submission("polygon is self-intersecting".into()));
            }
        }
    }
    Ok(())
}

fn segments_intersect(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    fn orient(p: [f64; 2], q: [f64; 2], r: [f64; 2]) -> f64 {
        (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
    }
    let o1 = orient(a, b, c);
    let o2 = orient(a, b, d);
    let o3 = orient(c, d, a);
    let o4 = orient(c, d, b);
    (o1 * o2 < 0.0) && (o3 * o4 < 0.0)
}

/// Maps a location in a bbox-mapped crop into its parent's coordinates. Returns the mapped
/// location and whether clipping happened. Wholly outside the parent is an error.
fn map_through_bbox(loc: &Location, m: &[f64; 4], parent: &str) -> Result<(Location, bool)> {
    let w = m[2] - m[0];
    let h = m[3] - m[1];
    let fwd = |x: f64, y: f64| [m[0] + x * w, m[1] + y * h];
    let mut out = loc.clone();
    out.frame = parent.to_string();
    let mut clipped = false;
    if let Some(b) = loc.bbox {
        let [x0, y0] = fwd(b[0], b[1]);
        let [x1, y1] = fwd(b[2], b[3]);
        if x1 <= 0.0 || y1 <= 0.0 || x0 >= 1.0 || y0 >= 1.0 {
            return Err(Error::Submission(format!(
                "location maps wholly outside its parent frame {parent:?}"
            )));
        }
        let c = [
            x0.clamp(0.0, 1.0),
            y0.clamp(0.0, 1.0),
            x1.clamp(0.0, 1.0),
            y1.clamp(0.0, 1.0),
        ];
        clipped = c != [x0, y0, x1, y1];
        out.bbox = Some(c);
    }
    if let Some(p) = &loc.polygon {
        let mapped: Vec<[f64; 2]> = p.iter().map(|v| fwd(v[0], v[1])).collect();
        let inside = mapped
            .iter()
            .any(|v| (0.0..=1.0).contains(&v[0]) && (0.0..=1.0).contains(&v[1]));
        if !inside {
            return Err(Error::Submission(format!(
                "polygon maps wholly outside its parent frame {parent:?}"
            )));
        }
        let clamped: Vec<[f64; 2]> = mapped
            .iter()
            .map(|v| [v[0].clamp(0.0, 1.0), v[1].clamp(0.0, 1.0)])
            .collect();
        clipped |= clamped != mapped;
        out.polygon = Some(clamped);
    }
    Ok((out, clipped))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_intersection_is_strict() {
        assert!(segments_intersect(
            [0.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
            [1.0, 0.0]
        ));
        assert!(!segments_intersect(
            [0.0, 0.0],
            [1.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0]
        ));
        // Touching at an endpoint does not count as crossing.
        assert!(!segments_intersect(
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0]
        ));
    }

    #[test]
    fn root_follows_parents_through_nested_crops() {
        let mut r = Registry::new();
        r.add_raw("c", 100, 100, Projection::Flat);
        r.add_crop(
            "crop:call:1:0",
            "raw:c",
            Mapping::Bbox([0.0, 0.0, 0.5, 0.5]),
            50,
            50,
        );
        r.add_crop(
            "crop:call:2:0",
            "crop:call:1:0",
            Mapping::Bbox([0.5, 0.5, 1.0, 1.0]),
            25,
            25,
        );
        assert_eq!(
            r.get("crop:call:2:0").map(|f| f.root().to_string()),
            Some("c".to_string())
        );
        let loc = Location {
            frame: "crop:call:2:0".into(),
            kind: LocationKind::Bbox,
            bbox: Some([0.0, 0.0, 1.0, 1.0]),
            polygon: None,
            yaw: None,
            pitch: None,
            fov: None,
        };
        let m = r.to_root(&loc).expect("maps");
        assert_eq!(m.frame, "raw:c");
        let b = m.bbox().expect("bbox");
        for (got, want) in b.iter().zip([0.25, 0.25, 0.5, 0.5]) {
            assert!((got - want).abs() < 1e-9, "{b:?}");
        }
    }
}
