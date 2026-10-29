// Equirectangular geometry for the viewer, the same maths as `squint_core::frames::sphere`:
// the direction a point of a square perspective view looks along, the equirectangular point of a
// direction, and the footprint on the panorama of a box or polygon drawn in such a view. The
// Rust side is the reference; `viewer_geometry` in the CLI tests holds this file to it.
//
// Conventions: normalised x runs from longitude -π (left) to π (right), y from latitude π/2
// (top) to -π/2 (bottom); yaw 0 is the centre column, increasing to the right; pitch 0 the
// horizon, increasing upward; fov the horizontal field of view of the square view.
(function (root) {
  'use strict';
  const rad = d => d * Math.PI / 180;

  function basis(view) {
    const sy = Math.sin(rad(view.yaw)), cy = Math.cos(rad(view.yaw));
    const sp = Math.sin(rad(view.pitch)), cp = Math.cos(rad(view.pitch));
    return {
      forward: [cp * cy, cp * sy, sp],
      right: [-sy, cy, 0],
      up: [-sp * cy, -sp * sy, cp],
    };
  }
  const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
  function normalise(v) {
    const n = Math.sqrt(dot(v, v));
    return n === 0 ? [1, 0, 0] : [v[0] / n, v[1] / n, v[2] / n];
  }

  /// The direction a normalised view point looks along.
  function direction(view, nx, ny) {
    const { forward: f, right: r, up: u } = basis(view);
    const t = Math.tan(rad(view.fov) / 2);
    const cx = (2 * nx - 1) * t, cy = (1 - 2 * ny) * t;
    return normalise([f[0] + cx * r[0] + cy * u[0], f[1] + cx * r[1] + cy * u[1], f[2] + cx * r[2] + cy * u[2]]);
  }

  /// The normalised view point of a direction, or null when it lies behind the view.
  function project(view, d) {
    const { forward: f, right: r, up: u } = basis(view);
    const depth = dot(d, f);
    if (depth <= 1e-9) return null;
    const t = Math.tan(rad(view.fov) / 2);
    const cx = dot(d, r) / depth / t, cy = dot(d, u) / depth / t;
    return [(cx + 1) / 2, (1 - cy) / 2];
  }

  function equirectOf(d) {
    const lon = Math.atan2(d[1], d[0]);
    const lat = Math.asin(Math.max(-1, Math.min(1, d[2])));
    let x = ((lon + Math.PI) / (2 * Math.PI)) % 1;
    if (x < 0) x += 1;
    return [x, (Math.PI / 2 - lat) / Math.PI];
  }

  function boundary(b, perEdge) {
    const out = [];
    for (let i = 0; i < perEdge; i++) { const t = i / perEdge; out.push([b[0] + t * (b[2] - b[0]), b[1]]); }
    for (let i = 0; i < perEdge; i++) { const t = i / perEdge; out.push([b[2], b[1] + t * (b[3] - b[1])]); }
    for (let i = 0; i < perEdge; i++) { const t = i / perEdge; out.push([b[2] - t * (b[2] - b[0]), b[3]]); }
    for (let i = 0; i < perEdge; i++) { const t = i / perEdge; out.push([b[0], b[3] - t * (b[3] - b[1])]); }
    return out;
  }

  /// Whether a normalised point lies in a region: a box by containment, the whole image by the
  /// unit square, a polygon by ray casting.
  function regionContains(loc, p) {
    if (loc.kind === 'bbox' && loc.bbox) return p[0] >= loc.bbox[0] && p[0] <= loc.bbox[2] && p[1] >= loc.bbox[1] && p[1] <= loc.bbox[3];
    if (loc.kind === 'whole_image') return p[0] >= 0 && p[0] <= 1 && p[1] >= 0 && p[1] <= 1;
    if (loc.kind === 'polygon' && loc.polygon) {
      const poly = loc.polygon; let inside = false;
      for (let i = 0; i < poly.length; i++) {
        const a = poly[i], b = poly[(i + 1) % poly.length];
        if ((a[1] > p[1]) !== (b[1] > p[1])) { const x = a[0] + (p[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0]); if (p[0] < x) inside = !inside; }
      }
      return inside;
    }
    return false;
  }

  /// The middle of a region in view coordinates: a box's centre, a polygon's vertex mean.
  function regionCentre(loc) {
    if (loc.kind === 'bbox' && loc.bbox) return [(loc.bbox[0] + loc.bbox[2]) / 2, (loc.bbox[1] + loc.bbox[3]) / 2];
    if (loc.kind === 'polygon' && loc.polygon && loc.polygon.length) {
      const n = loc.polygon.length;
      return [loc.polygon.reduce((s, v) => s + v[0], 0) / n, loc.polygon.reduce((s, v) => s + v[1], 0) / n];
    }
    return [0.5, 0.5];
  }

  /// The equirectangular footprint of a region's boundary points in a view: { kind: 'polygon',
  /// polygon, clipped } cut at the seam when consecutive samples jump across it, or
  /// { kind: 'bbox', bbox } across the full width when the region holds a pole.
  function footprint(view, points, loc) {
    const mapped = points.map(p => equirectOf(direction(view, p[0], p[1])));
    const centre = regionCentre(loc);
    const centreX = equirectOf(direction(view, centre[0], centre[1]))[0];
    const holdsPole = pole => { const p = project(view, pole); return p !== null && regionContains(loc, p); };
    const ys = mapped.map(m => m[1]);
    if (holdsPole([0, 0, 1])) return { kind: 'bbox', bbox: [0, 0, 1, Math.max(1e-6, Math.max(...ys))], clipped: false };
    if (holdsPole([0, 0, -1])) return { kind: 'bbox', bbox: [0, Math.min(1 - 1e-6, Math.min(...ys)), 1, 1], clipped: false };
    const straddles = mapped.length > 1 && mapped.some((m, i) => Math.abs(m[0] - mapped[(i + 1) % mapped.length][0]) > 0.5);
    if (!straddles) return { kind: 'polygon', polygon: mapped, clipped: false };
    const edge = centreX < 0.5 ? 0 : 1;
    return { kind: 'polygon', polygon: mapped.map(m => (Math.abs(m[0] - centreX) > 0.5 ? [edge, m[1]] : m)), clipped: true };
  }

  /// Maps a location drawn in a view (a bbox, polygon or whole_image in normalised view
  /// coordinates) to the panorama. Returns { kind, bbox, polygon, clipped } or null.
  function mapThroughView(loc, view) {
    let points;
    if (loc.kind === 'bbox' && loc.bbox) points = boundary(loc.bbox, 16);
    else if (loc.kind === 'whole_image') points = boundary([0, 0, 1, 1], 16);
    else if (loc.kind === 'polygon' && loc.polygon) {
      const perEdge = Math.max(1, Math.floor(64 / Math.max(1, loc.polygon.length)));
      points = [];
      loc.polygon.forEach((a, i) => {
        const b = loc.polygon[(i + 1) % loc.polygon.length];
        for (let k = 0; k < perEdge; k++) { const t = k / perEdge; points.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]); }
      });
    } else return null;
    return footprint(view, points, loc);
  }

  /// The inverse of a row-major 3×3 matrix.
  function invert3(m) {
    const det = m[0] * (m[4] * m[8] - m[5] * m[7]) - m[1] * (m[3] * m[8] - m[5] * m[6]) + m[2] * (m[3] * m[7] - m[4] * m[6]);
    if (!isFinite(det) || Math.abs(det) < 1e-300) return null;
    return [
      (m[4] * m[8] - m[5] * m[7]) / det, (m[2] * m[7] - m[1] * m[8]) / det, (m[1] * m[5] - m[2] * m[4]) / det,
      (m[5] * m[6] - m[3] * m[8]) / det, (m[0] * m[8] - m[2] * m[6]) / det, (m[2] * m[3] - m[0] * m[5]) / det,
      (m[3] * m[7] - m[4] * m[6]) / det, (m[1] * m[6] - m[0] * m[7]) / det, (m[0] * m[4] - m[1] * m[3]) / det,
    ];
  }
  const applyH = (h, p) => { const d = h[6] * p[0] + h[7] * p[1] + h[8]; return [(h[0] * p[0] + h[1] * p[1] + h[2]) / d, (h[3] * p[0] + h[4] * p[1] + h[5]) / d]; };
  const cornersOf = loc => (loc.kind === 'bbox' && loc.bbox ? [[loc.bbox[0], loc.bbox[1]], [loc.bbox[2], loc.bbox[1]], [loc.bbox[2], loc.bbox[3]], [loc.bbox[0], loc.bbox[3]]]
    : loc.kind === 'whole_image' ? [[0, 0], [1, 0], [1, 1], [0, 1]] : loc.kind === 'polygon' && loc.polygon ? loc.polygon : null);
  const clampAll = pts => { let clipped = false; const out = pts.map(([x, y]) => { const cx = Math.min(1, Math.max(0, x)), cy = Math.min(1, Math.max(0, y)); if (cx !== x || cy !== y) clipped = true; return [cx, cy]; }); return { out, clipped }; };

  /// A location on an aligned flat frame (its parent's coordinates) back to the image's raw
  /// frame through the inverse homography: the polygon of its corners.
  function mapThroughHomography(loc, params) {
    const inv = invert3(params);
    const pts = cornersOf(loc);
    if (!inv || !pts) return null;
    const { out, clipped } = clampAll(pts.map(p => applyH(inv, p)));
    return { kind: 'polygon', polygon: out, clipped };
  }

  /// A location on an aligned panorama back to the image's raw frame through the inverse rotation:
  /// the footprint of its boundary.
  function mapThroughRotation(loc, params) {
    const rt = [params[0], params[3], params[6], params[1], params[4], params[7], params[2], params[5], params[8]];
    const rot = p => [rt[0] * p[0] + rt[1] * p[1] + rt[2] * p[2], rt[3] * p[0] + rt[4] * p[1] + rt[5] * p[2], rt[6] * p[0] + rt[7] * p[1] + rt[8] * p[2]];
    let samples;
    if (loc.kind === 'bbox' && loc.bbox) samples = boundary(loc.bbox, 16);
    else if (loc.kind === 'whole_image') samples = boundary([0, 0, 1, 1], 16);
    else if (loc.kind === 'polygon' && loc.polygon) samples = loc.polygon;
    else return null;
    const dirs = samples.map(p => rot(directionOfEquirect(p[0], p[1])));
    const centre = dirs.reduce((a, d) => [a[0] + d[0], a[1] + d[1], a[2] + d[2]], [0, 0, 0]);
    return footprintOfDirections(dirs, centre);
  }

  function directionOfEquirect(nx, ny) {
    const lon = nx * 2 * Math.PI - Math.PI, lat = Math.PI / 2 - ny * Math.PI;
    return [Math.cos(lat) * Math.cos(lon), Math.cos(lat) * Math.sin(lon), Math.sin(lat)];
  }

  /// The footprint of a closed boundary of directions facing `centre`, as the Rust side does it:
  /// a pole is held when the longitudes cover the circle with no gap of a quarter turn.
  function footprintOfDirections(dirs, centre) {
    const mapped = dirs.map(equirectOf);
    const centreX = equirectOf(normalise(centre))[0];
    const winds = sign => {
      if (dirs.length < 4 || centre[2] * sign <= 0) return false;
      const lons = dirs.map(d => Math.atan2(d[1], d[0])).sort((a, b) => a - b);
      let gap = lons[0] + 2 * Math.PI - lons[lons.length - 1];
      for (let i = 1; i < lons.length; i++) gap = Math.max(gap, lons[i] - lons[i - 1]);
      return gap < Math.PI / 2;
    };
    const ys = mapped.map(m => m[1]);
    if (winds(1)) return { kind: 'bbox', bbox: [0, 0, 1, Math.max(1e-6, Math.max(...ys))], clipped: false };
    if (winds(-1)) return { kind: 'bbox', bbox: [0, Math.min(1 - 1e-6, Math.min(...ys)), 1, 1], clipped: false };
    const straddles = mapped.length > 1 && mapped.some((m, i) => Math.abs(m[0] - mapped[(i + 1) % mapped.length][0]) > 0.5);
    if (!straddles) return { kind: 'polygon', polygon: mapped, clipped: false };
    const edge = centreX < 0.5 ? 0 : 1;
    return { kind: 'polygon', polygon: mapped.map(m => (Math.abs(m[0] - centreX) > 0.5 ? [edge, m[1]] : m)), clipped: true };
  }

  root.squintGeometry = { direction, project, equirectOf, boundary, footprint, mapThroughView, mapThroughHomography, mapThroughRotation };
})(typeof globalThis !== 'undefined' ? globalThis : this);
