// Unit tests for the launcher's routing/ranking logic (PLAN §5.7.2).
import {test, assert, assertEq, finish} from '../../testing/harness.js';
import {
    calculatorExpression, parseQalcOutput, parseContextHits,
    mergeResults, parseResultId,
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

finish('launcher ranking');
