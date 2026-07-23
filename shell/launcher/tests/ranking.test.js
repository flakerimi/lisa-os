// Unit tests for the launcher's routing/ranking logic (PLAN §5.7.2).
import {test, assert, assertEq, finish} from '../../testing/harness.js';
import {
    calculatorExpression, parseQalcOutput, parseContextHits,
    looksLikeQuestion, mergeResults, parseResultId,
} from '../lib/ranking.js';

test('arithmetic routes to qalc, never the model', () => {
    assertEq(calculatorExpression('2+2'), '2+2');
    assertEq(calculatorExpression(' (14.5*3) / 2 '), '(14.5*3) / 2');
    assertEq(calculatorExpression('2^10'), '2^10');
});

test('unit conversions route to qalc', () => {
    assertEq(calculatorExpression('12 km to miles'), '12 km to miles');
    assertEq(calculatorExpression('3.5 kg in lb'), '3.5 kg to lb');
});

test('= prefix is the explicit escape hatch', () => {
    assertEq(calculatorExpression('= sqrt(2)'), 'sqrt(2)');
});

test('plain text and bare numbers are not calculator queries', () => {
    assertEq(calculatorExpression('rotate this pdf'), null);
    assertEq(calculatorExpression('meeting notes 2026'), null);
    assertEq(calculatorExpression('42'), null);
    assertEq(calculatorExpression('3.14'), null);
    assertEq(calculatorExpression(''), null);
    assertEq(calculatorExpression(null), null);
});

test('parseQalcOutput takes the last non-empty line', () => {
    assertEq(parseQalcOutput('4\n'), '4');
    assertEq(parseQalcOutput('warning: assuming radians\n0.909297\n'), '0.909297');
    assertEq(parseQalcOutput('\n\n'), null);
    assertEq(parseQalcOutput(null), null);
});

test('parseContextHits reads the CLI format', () => {
    const hits = parseContextHits('[file] /home/u/a.md\n    alpha beta\n');
    assertEq(hits, [{provenance: 'file', source: '/home/u/a.md', snippet: 'alpha beta'}]);
});

test('mergeResults orders calc, actions, then deduped files', () => {
    const ids = mergeResults({
        calc: {expression: '2+2', result: '4'},
        actions: [],
        files: [
            {provenance: 'file', source: '/a.md', snippet: 's1'},
            {provenance: 'file', source: '/a.md', snippet: 's2'},
            {provenance: 'file', source: '/b.md', snippet: 's3'},
        ],
    }, 10);
    assertEq(ids, ['calc:2+2=4', 'file:/a.md', 'file:/b.md']);
});

test('mergeResults respects the cap and empty lanes', () => {
    const files = [
        {provenance: 'file', source: '/a', snippet: ''},
        {provenance: 'file', source: '/b', snippet: ''},
        {provenance: 'file', source: '/c', snippet: ''},
    ];
    assertEq(mergeResults({files}, 2), ['file:/a', 'file:/b']);
    assertEq(mergeResults({}, 5), []);
    assertEq(mergeResults(null, 5), []);
});

test('parseResultId inverts mergeResults ids', () => {
    assertEq(parseResultId('calc:2+2=4'), {kind: 'calc', expression: '2+2', result: '4'});
    assertEq(parseResultId('calc:1=2=3'), {kind: 'calc', expression: '1=2', result: '3'});
    assertEq(parseResultId('file:/home/u/x y.md'), {kind: 'file', path: '/home/u/x y.md'});
    assertEq(parseResultId('action:rotate-pdf'), {kind: 'action', name: 'rotate-pdf'});
    assertEq(parseResultId('bogus'), null);
});

test('looksLikeQuestion: question shapes promote the ask lane', () => {
    assert(looksLikeQuestion('how do I update the system'));
    assert(looksLikeQuestion('capital of france?'));
    assert(looksLikeQuestion('What is the ledger'));
    assert(looksLikeQuestion("what's on my calendar"));
    assert(looksLikeQuestion('tell me a joke'));
    assert(looksLikeQuestion('meeting notes from last week'));
});

test('looksLikeQuestion: terse lookups do not promote', () => {
    assert(!looksLikeQuestion('notes'));
    assert(!looksLikeQuestion('2+2'));
    assert(!looksLikeQuestion('settings wi-fi'));
    assert(!looksLikeQuestion(''));
    assert(!looksLikeQuestion(null));
});

test('ask lane: promoted questions sit right after the calc slot', () => {
    const ids = mergeResults({
        files: [{provenance: 'file', source: '/a.md', snippet: 's'}],
        ask: 'what is the capital of france',
    }, 8);
    assertEq(ids, ['ask:what is the capital of france', 'file:/a.md']);
});

test('ask lane: calc keeps first place, unpromoted ask goes last', () => {
    const ids = mergeResults({
        calc: {expression: '2+2', result: '4'},
        files: [{provenance: 'file', source: '/a.md', snippet: 's'}],
        ask: '2+2',
    }, 8);
    assertEq(ids, ['calc:2+2=4', 'file:/a.md', 'ask:2+2']);
});

test('ask lane: promoted ask still yields to calc', () => {
    const ids = mergeResults({
        calc: {expression: '12 km to miles', result: '7.4564543 mile'},
        ask: 'how many miles is 12 km?',
    }, 8);
    assertEq(ids, ['calc:12 km to miles=7.4564543 mile', 'ask:how many miles is 12 km?']);
});

test('ask lane: absent when no query, trimmed, and capped like any lane', () => {
    assertEq(mergeResults({ask: ''}, 8), []);
    assertEq(mergeResults({ask: '  '}, 8), []);
    assertEq(mergeResults({ask: ' hi '}, 8), ['ask:hi']);
    // An unpromoted ask is the first thing the cap sheds.
    const files = Array.from({length: 8}, (_, i) =>
        ({provenance: 'file', source: `/${i}`, snippet: ''}));
    assertEq(mergeResults({files, ask: 'x'}, 8).includes('ask:x'), false);
    assert(mergeResults({files, ask: 'why x?'}, 8).includes('ask:why x?'));
});

test('parseResultId handles ask ids, including queries with colons', () => {
    assertEq(parseResultId('ask:what is rust: ownership'),
        {kind: 'ask', query: 'what is rust: ownership'});
    assertEq(parseResultId('ask:'), {kind: 'ask', query: ''});
});

finish('launcher ranking');
