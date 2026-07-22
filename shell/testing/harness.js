// Minimal test harness for the shell surfaces' pure-logic modules
// (docs/PLAN.md §11: unit per component). Runtime-agnostic on purpose:
// runs under gjs -m (Linux/CI), node (CI), and jsc -m (macOS dev
// hosts, /System/.../JavaScriptCore.framework/Helpers/jsc) — see
// `just shell-test`. A failing assertion throws; the runner treats a
// non-zero exit as failure.

const log = globalThis.console?.log?.bind(globalThis.console) ?? globalThis.print;

let failures = 0;
let passes = 0;

export function test(name, fn) {
    try {
        fn();
        passes += 1;
        log(`  ok    ${name}`);
    } catch (e) {
        failures += 1;
        log(`  FAIL  ${name}: ${e.message}`);
    }
}

export function assertEq(actual, expected, msg = '') {
    const a = JSON.stringify(actual);
    const b = JSON.stringify(expected);
    if (a !== b)
        throw new Error(`${msg} expected ${b}, got ${a}`);
}

export function assert(cond, msg = 'assertion failed') {
    if (!cond)
        throw new Error(msg);
}

export function finish(suite) {
    log(`${suite}: ${passes} passed, ${failures} failed`);
    if (failures > 0) {
        // Non-zero exit on every supported runtime.
        if (globalThis.imports?.system)
            globalThis.imports.system.exit(1);
        if (globalThis.process?.exit)
            globalThis.process.exit(1);
        throw new Error(`${failures} test(s) failed`);
    }
}
