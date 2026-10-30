// The funnel of one run as rows for a waterfall: each step of the journal with its start relative
// to the first event, its duration and its outcome. Steps that span two events (a model turn from
// `turn_sent` to `turn_received`, a tool call from `tool_called` to `tool_returned`) become one
// row; the rest are points whose duration is the time since the event before them. Pure, so
// `viewer_timeline` in the CLI tests can hold it to a fixed journal under node.
(function (root) {
  'use strict';

  function rows(events) {
    if (!events.length) return [];
    const t0 = events[0].unix_ms;
    const out = [];
    const turns = new Map();
    const calls = new Map();
    let prev = t0;
    const point = (e, label, status, detail) => {
      out.push({ step: e.event, label, start_ms: prev - t0, duration_ms: e.unix_ms - prev, status, detail: detail || '' });
    };
    for (const e of events) {
      const d = e.data || {};
      switch (e.event) {
        case 'turn_sent': {
          const row = { step: 'turn', label: `turn ${d.turn}`, start_ms: e.unix_ms - t0, duration_ms: 0, status: 'pending', detail: '', cost_usd: null };
          turns.set(d.turn, row);
          out.push(row);
          break;
        }
        case 'turn_received': {
          const row = turns.get(d.turn);
          if (row) {
            row.duration_ms = e.unix_ms - t0 - row.start_ms;
            row.status = 'ok';
            row.cost_usd = d.cost_usd ?? null;
            const calls_ = (d.tool_calls || []).join(', ');
            row.detail = `${d.stop_reason || ''}${calls_ ? `: ${calls_}` : ''}`;
          }
          break;
        }
        case 'attempt_failed': {
          const row = turns.get(d.turn);
          if (row) {
            row.duration_ms = e.unix_ms - t0 - row.start_ms;
            row.status = 'failed';
            row.detail = `${d.class || 'error'} (attempt ${d.attempts || 1})`;
          }
          break;
        }
        case 'tool_called': {
          const id = d.call_id;
          const open = id && calls.get(id);
          if (open && d.invalid_args) {
            // The tool itself refused the arguments: the dispatch line is closed by this one.
            open.duration_ms = e.unix_ms - t0 - open.start_ms;
            open.status = 'invalid_args';
            open.detail = d.invalid_args;
          } else if (d.executed) {
            const row = { step: 'tool', label: `${d.tool} ${id}`, start_ms: e.unix_ms - t0, duration_ms: 0, status: 'pending', detail: '' };
            calls.set(id, row);
            out.push(row);
          } else {
            out.push({ step: 'tool', label: `${d.tool}${id ? ` ${id}` : ''}`, start_ms: e.unix_ms - t0, duration_ms: 0, status: d.invalid_args ? 'invalid_args' : 'not_run', detail: d.invalid_args || d.reason || '' });
          }
          break;
        }
        case 'tool_returned': {
          const row = calls.get(d.call_id);
          if (row) {
            row.duration_ms = e.unix_ms - t0 - row.start_ms;
            row.status = d.status || 'ok';
            const imgs = (d.images || []).length;
            row.detail = `${d.regions || 0} region(s), ${imgs} image(s), ${(d.added || []).length} item(s) added`;
          }
          break;
        }
        case 'resolved': point(e, 'resolve', 'ok', `${(d.images || []).length} image(s), ${(d.instances || []).length} instance(s)`); break;
        case 'prepared': point(e, 'prepare', 'ok', `${d.images || 0} image(s) prepared`); break;
        case 'reserved': point(e, 'reserve', 'ok', d.envelope ? `envelope $${Number(d.envelope.usd).toFixed(4)}` : ''); break;
        case 'repair': point(e, 'repair', 'warn', `${d.form || ''}${d.reason ? `: ${d.reason}` : ''}`); break;
        case 'submission': point(e, 'submission', 'ok', ''); break;
        case 'verdict': point(e, 'verdict', 'ok', d.verdict || d.computed || ''); break;
        case 'error': point(e, 'error', 'failed', `${d.class || ''}: ${d.message || ''}`); break;
        case 'cancelled': point(e, 'cancelled', 'failed', ''); break;
        default: break;
      }
      prev = e.unix_ms;
    }
    return out;
  }

  function parse(text) {
    return text.split('\n').filter(l => l.trim()).map(l => JSON.parse(l));
  }

  root.squintTimeline = { rows, parse };
})(typeof globalThis !== 'undefined' ? globalThis : this);
