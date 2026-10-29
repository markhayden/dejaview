// Equirectangular geometry for the viewer, the same maths as `dejaview_core::frames::sphere`:
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

  /// The equirectangular footprint of view points: { kind: 'polygon', polygon, clipped } cut at
  /// the seam when it straddles it, or { kind: 'bbox', bbox } across the full width when the view
  /// holds a pole.
  function footprint(view, points) {
    const mapped = points.map(p => equirectOf(direction(view, p[0], p[1])));
    const centreX = equirectOf(direction(view, 0.5, 0.5))[0];
    const holdsPole = pole => {
      const p = project(view, pole);
      return p !== null && p[0] >= 0 && p[0] <= 1 && p[1] >= 0 && p[1] <= 1;
    };
    const ys = mapped.map(m => m[1]);
    if (holdsPole([0, 0, 1])) return { kind: 'bbox', bbox: [0, 0, 1, Math.max(1e-6, Math.max(...ys))], clipped: false };
    if (holdsPole([0, 0, -1])) return { kind: 'bbox', bbox: [0, Math.min(1 - 1e-6, Math.min(...ys)), 1, 1], clipped: false };
    const straddles = mapped.some(m => Math.abs(m[0] - centreX) > 0.5);
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
    return footprint(view, points);
  }

  root.dejaviewGeometry = { direction, project, equirectOf, boundary, footprint, mapThroughView };
})(typeof globalThis !== 'undefined' ? globalThis : this);
