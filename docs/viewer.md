# Report viewer

The static viewer under `crates/dejaview-cli/viewer/` (`index.html`, `viewer.js`, `viewer.css`):
plain HTML and vanilla JavaScript, no build step, embedded in the binary so an installed
`dejaview` carries it. Equirectangular overlays arrive in task 4.2b.

## Serving a run

```
dejaview report view <decision dir> [--port 0] [--open]
```

Binds `127.0.0.1:<port>` (0 picks a free port), prints the URL, and serves until stopped. With
`--open` the system browser is launched on it. The directory must hold `report.json`
(`dejaview report rebuild` makes one from the journals).

The server answers `GET` and `HEAD` only, for exactly these paths:

| Path | Content |
|---|---|
| `/`, `/viewer.js`, `/viewer.css` | the embedded viewer |
| `/report.json` | the decision's report, byte for byte |
| `/run-<n>/artifacts/<sha256>` | a run artifact: an image the model saw, a request or response body, a tool envelope; typed by its first bytes |

Everything else is `404`, so nothing outside the decision directory (and nothing in it beyond the
report and the artifacts) can be read through it. A request whose `Host` header is not
`127.0.0.1:<port>` or `localhost:<port>` is refused with `421` before any route, so a page from
elsewhere cannot reach the report through a name rebound to loopback. Responses carry
`Cache-Control: no-store`, `X-Content-Type-Options: nosniff`, `Content-Security-Policy:
default-src 'self'` and `Connection: close`. The page itself requests nothing but these paths
from the server that served it: no fonts, scripts or images from anywhere else.

## What it shows

The viewer declares the `report_version`s it reads (`SUPPORTED_REPORT_VERSIONS` in `viewer.js`,
currently `[1]`) and refuses others with a message. The version names the shape a reader must
accept: fields added since (`frames`, `findings`) default when absent, so a current binary reads
every version 1 report; an older binary refuses a newer report rather than misread it. For a supported report:

- **Decision**: verdict, exit code, use case, provider, tools, decision runs and agreement, the
  policy, spent against envelope and cap, cache status, prompt and harness versions; a row per
  run; the rules, job and resolved config folded away.
- **Images the model saw**, per run: every frame in `images_sent` with its size, projection and,
  for a crop, its parent and mapping box. Every location is drawn twice: on the frame it names, as
  submitted, and on its root image in **raw coordinates**, mapped through the frame tree
  (`frames` in the report: raw frames and every crop a tool returned, with the box each crop
  covers of its parent). Checklist items, findings (violations in red, observed and allowed
  findings grey) and tool regions (from each call's envelope) have their own colours;
  `whole_image` is a border, `absent` a dashed border, a location clipped at its parent is dotted.
  A `view` location has no box mapping until equirectangular support lands and is listed as not
  drawn.
- **Tool results**: every call with status, duration, arguments, the items it added or dropped,
  the envelope's summary and region count, and a link to the envelope.
- **Checklist and submission**: the checklist as it stood at the end, joined with each item's
  disposition, confidence, reason and location; the findings.
- **Aggregation**: each item's aggregation row and the finding rows.
- **Turns**: stop reason, tool calls, attempts, cost against bound, usage, history membership,
  provider request id, links to the request and response bodies; repairs; the error if any;
  obligation instances folded away.

Overlays are SVG with a unit viewBox over the image, so a box at `[0.1, 0.1, 0.3, 0.3]` sits at
those fractions of the displayed image whatever its size; the viewer test checks a crop-frame
location lands at the mapped position on the candidate.

## Tests

`cargo test -p dejaview --features testkit --test viewer_serve` runs a decision with a returned
crop frame and checks the server's routes, types and refusals. The offline smoke
(`scripts/offline-smoke.sh`, run on every packaged and released binary) starts `report view` on a
minimal directory and fetches the page and the report. The embedded assets are covered by
`scripts/check-packaging.sh`.

## Devtools check (2026-10-27)

On the decision the viewer test produces (`fixtures/scripts/one-call-crop-finding.json` with
`fixtures/tool-scripts/crop-image.json`: a tool returns a crop covering `[0, 0, 0.5, 0.5]` of the
candidate and the model reports a violation at `[0.2, 0.2, 0.6, 0.6]` of that crop), served by
`report view` and opened in Chrome:

- console: no messages;
- network: seven requests, all to `http://127.0.0.1:<port>/` (page, stylesheet, script,
  `report.json`, three artifacts);
- the finding is drawn on the crop panel at fractions `[0.2, 0.2, 0.6, 0.6]` of the image and on
  the candidate panel at `[0.1, 0.1, 0.3, 0.3]`, which is where the report's `root` location puts
  it, measured from the shapes' bounding boxes against the images'.
