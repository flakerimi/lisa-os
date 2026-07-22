// Semantic launcher — pure query/result logic (PLAN §5.7.2).
//
// One box mixing app launch (GNOME's own provider keeps that lane),
// file hits from the Context Fabric, bus actions (M5), and
// calculator/unit answers. The spec's hard rule lives here: math is
// *routed to qalc*, the model never does arithmetic. Pure logic — no
// GNOME imports — so it unit-tests under gjs/node/jsc.

/**
 * Should this query go to qalc? Deliberately conservative: a stray
 * calculator hit above real results is worse than none.
 * Accepted shapes:
 *   - explicit "= <anything>" prefix (power-user escape hatch)
 *   - pure arithmetic: digits + operators/parens, at least one of each
 *   - unit/currency conversion: "12 km to miles", "3.5 kg in lb"
 *
 * @param {string} query
 * @returns {string|null} the expression to hand qalc, or null
 */
export function calculatorExpression(query) {
    const q = (query ?? '').trim();
    if (q.startsWith('=') && q.length > 1)
        return q.slice(1).trim();
    if (/^[\d\s+\-*/^().,%!]+$/.test(q) && /\d/.test(q) && /[+\-*/^%]/.test(q) &&
        !/^[\d.,\s]+$/.test(q))
        return q;
    const conversion = q.match(/^(\d+(?:[.,]\d+)?\s*\S+)\s+(?:to|in)\s+(\S+)$/i);
    if (conversion)
        return `${conversion[1]} to ${conversion[2]}`;
    return null;
}

/**
 * qalc -t prints the terse result on stdout (possibly with warnings on
 * preceding lines). Last non-empty line is the answer.
 *
 * @param {string} stdout
 * @returns {string|null}
 */
export function parseQalcOutput(stdout) {
    const lines = (stdout ?? '').split('\n').map(l => l.trim()).filter(l => l !== '');
    return lines.length > 0 ? lines[lines.length - 1] : null;
}

/**
 * Parse `lisa context search` CLI output (same format the overlay
 * parses; kept local so the extension directory is self-contained).
 *
 * @param {string} text
 * @returns {{provenance: string, source: string, snippet: string}[]}
 */
export function parseContextHits(text) {
    const hits = [];
    let current = null;
    for (const line of (text ?? '').split('\n')) {
        const head = line.match(/^\[([^\]]+)\]\s+(.+)$/);
        if (head) {
            current = {provenance: head[1], source: head[2].trim(), snippet: ''};
            hits.push(current);
        } else if (current && line.trim() !== '') {
            current.snippet += (current.snippet ? '\n' : '') + line.trim();
        }
    }
    return hits;
}

/**
 * Merge result lanes into the ordered id list the Shell provider
 * returns. Order (spec): calculator answer first (it is definitionally
 * exact), then bus actions (M5, always empty today), then file hits —
 * lexical first; semantic refinement replaces/extends them when the
 * embedding pipeline lands (§5.3). Files dedupe by source path.
 *
 * Result ids are self-describing (parseResultId inverts them) so
 * activateResult needs no side table.
 *
 * @param {{calc?: {expression: string, result: string}|null,
 *          actions?: string[],
 *          files?: {provenance: string, source: string, snippet: string}[]}} lanes
 * @param {number} max
 * @returns {string[]}
 */
export function mergeResults(lanes, max) {
    const ids = [];
    if (lanes?.calc)
        ids.push(`calc:${lanes.calc.expression}=${lanes.calc.result}`);
    for (const a of lanes?.actions ?? [])
        ids.push(`action:${a}`);
    const seen = new Set();
    for (const hit of lanes?.files ?? []) {
        if (seen.has(hit.source))
            continue;
        seen.add(hit.source);
        ids.push(`file:${hit.source}`);
    }
    return ids.slice(0, max);
}

/**
 * @param {string} id
 * @returns {{kind: 'calc', expression: string, result: string} |
 *           {kind: 'file', path: string} |
 *           {kind: 'action', name: string} | null}
 */
export function parseResultId(id) {
    if (id.startsWith('calc:')) {
        const body = id.slice(5);
        const eq = body.lastIndexOf('=');
        if (eq < 0)
            return null;
        return {kind: 'calc', expression: body.slice(0, eq), result: body.slice(eq + 1)};
    }
    if (id.startsWith('file:'))
        return {kind: 'file', path: id.slice(5)};
    if (id.startsWith('action:'))
        return {kind: 'action', name: id.slice(7)};
    return null;
}
