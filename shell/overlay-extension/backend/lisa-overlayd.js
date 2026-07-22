#!/usr/bin/env -S gjs -m
// lisa-overlayd — headless assistant-overlay backend (PLAN §5.7.1).
//
// Owns org.lisa.Overlay1 on the session bus: state and token streams
// live here so frontends stay thin — the GNOME Shell extension and the
// wlr-layer-shell client (Track L) both just render this service.
//
// Per Ask():
//   1. [my stuff] → `lisa context search` (Context Fabric, PLAN §5.3;
//      retrieval is ledgered by the CLI) → provenance-fenced envelope
//      (Appendix C via lib/envelope.js).
//   2. org.lisa.Inference1.OpenSession → (session path, token fd);
//      Session.Generate; tokens are read off the fd and re-emitted as
//      Token signals until EOF ⇒ Finished. Every generation is
//      ledgered by lisa-inferenced (dataflow rule 4).
// [this window] (§5.7.4, M6) and [selection] (§5.7.3 layer 3) are
// reported unavailable in Started meta until their layers land.
//
// M5 turns this into an Agent Bus (MCP) client; the D-Bus surface is
// designed to survive that swap unchanged.

import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import {buildEnvelope, parseContextHits, classifyAffordances}
    from '../lib/envelope.js';
import {OVERLAY_IFACE_XML, OVERLAY_BUS_NAME, OVERLAY_OBJECT_PATH}
    from '../lib/iface.js';

const INFERENCE_BUS = 'org.lisa.Inference1';
const INFERENCE_PATH = '/org/lisa/Inference1';
const INFERENCE_IFACE = 'org.lisa.Inference1';
const SESSION_IFACE = 'org.lisa.Inference1.Session';
const CONTEXT_HITS = 3;

Gio._promisify(Gio.Subprocess.prototype, 'communicate_utf8_async');
Gio._promisify(Gio.InputStream.prototype, 'read_bytes_async');
Gio._promisify(Gio.DBusConnection.prototype, 'call');

class OverlayService {
    constructor(connection) {
        this._connection = connection;
        this._impl = Gio.DBusExportedObject.wrapJSObject(OVERLAY_IFACE_XML, this);
        this._impl.export(connection, OVERLAY_OBJECT_PATH);
        this._nextId = 1;
        this._active = null; // {id, cancellable, sessionPath, stream}
    }

    // ---- D-Bus methods -------------------------------------------------

    Ask(prompt, options) {
        const id = this._nextId++;
        const opts = this._unpackOptions(options);
        // Fire and stream; errors surface as Finished("error", detail).
        this._run(id, prompt, opts).catch(e => {
            this._finish(id, 'error', String(e?.message ?? e));
        });
        return id;
    }

    Cancel(queryId) {
        const active = this._active;
        if (!active || active.id !== Number(queryId))
            return;
        active.cancelled = true;
        active.cancellable.cancel();
        if (active.sessionPath)
            this._sessionCall(active.sessionPath, 'Cancel', null).catch(() => {});
    }

    GetStatus() {
        return {
            state: GLib.Variant.new_string(this._active ? 'streaming' : 'idle'),
            active_query: GLib.Variant.new_uint64(this._active?.id ?? 0),
        };
    }

    // ---- internals -----------------------------------------------------

    _unpackOptions(options) {
        const out = {};
        for (const key of ['my_stuff', 'window', 'selection', 'model_hint']) {
            const v = options[key];
            if (v !== undefined)
                out[key] = v instanceof GLib.Variant ? v.recursiveUnpack() : v;
        }
        return out;
    }

    async _run(id, prompt, opts) {
        const cancellable = new Gio.Cancellable();
        this._active = {id, cancellable, sessionPath: null, cancelled: false};

        const {wanted, unavailable} = classifyAffordances(opts);
        let hits = [];
        if (wanted.includes('my_stuff'))
            hits = await this._searchContext(prompt, cancellable);

        this._emit('Started', new GLib.Variant('(ts)', [id, JSON.stringify({
            sources: hits.map(h => ({provenance: h.provenance, source: h.source})),
            unavailable,
        })]));

        const envelope = buildEnvelope(prompt, hits);
        const {sessionPath, stream} = await this._openSession(opts.model_hint);
        this._active.sessionPath = sessionPath;
        this._active.stream = stream;

        try {
            const params = {priority: GLib.Variant.new_string('interactive')};
            await this._sessionCall(sessionPath, 'Generate',
                new GLib.Variant('(sa{sv})', [envelope, params]));

            const decoder = new TextDecoder('utf-8');
            for (;;) {
                const bytes = await stream.read_bytes_async(
                    4096, GLib.PRIORITY_DEFAULT, cancellable);
                if (bytes.get_size() === 0)
                    break; // EOF = end-of-message (§5.1).
                this._emit('Token', new GLib.Variant('(ts)',
                    [id, decoder.decode(bytes.toArray(), {stream: true})]));
            }
            this._finish(id, this._active?.cancelled ? 'cancelled' : 'ok', '');
        } catch (e) {
            if (this._active?.cancelled)
                this._finish(id, 'cancelled', '');
            else
                throw e;
        } finally {
            stream.close(null);
            this._sessionCall(sessionPath, 'Close', null).catch(() => {});
        }
    }

    async _searchContext(query, cancellable) {
        try {
            const argv = [this._lisaCli(), 'context', 'search', query,
                '--limit', String(CONTEXT_HITS)];
            const proc = Gio.Subprocess.new(argv,
                Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_PIPE);
            const [stdout] = await proc.communicate_utf8_async(null, cancellable);
            if (!proc.get_successful())
                return [];
            return parseContextHits(stdout);
        } catch (e) {
            logError(e, 'context search failed; answering without [my stuff]');
            return [];
        }
    }

    _lisaCli() {
        return GLib.getenv('LISA_CLI') ?? 'lisa';
    }

    async _openSession(modelHint) {
        const options = {};
        if (modelHint)
            options.model_hint = GLib.Variant.new_string(modelHint);
        const [ret, fdList] = await new Promise((resolve, reject) => {
            this._connection.call_with_unix_fd_list(
                INFERENCE_BUS, INFERENCE_PATH, INFERENCE_IFACE, 'OpenSession',
                new GLib.Variant('(a{sv})', [options]),
                new GLib.VariantType('(oh)'),
                Gio.DBusCallFlags.NONE, -1, null, null,
                (conn, res) => {
                    try {
                        resolve(conn.call_with_unix_fd_list_finish(res));
                    } catch (e) {
                        reject(e);
                    }
                });
        });
        const [sessionPath, fdIndex] = ret.deepUnpack();
        const fd = fdList.get(fdIndex); // dup'd; the stream owns it now.
        const stream = new Gio.UnixInputStream({fd, close_fd: true});
        return {sessionPath, stream};
    }

    _sessionCall(sessionPath, method, args) {
        return this._connection.call(
            INFERENCE_BUS, sessionPath, SESSION_IFACE, method, args,
            null, Gio.DBusCallFlags.NONE, -1, null);
    }

    _finish(id, status, detail) {
        if (this._active?.id === id)
            this._active = null;
        this._emit('Finished', new GLib.Variant('(tss)', [id, status, detail]));
    }

    _emit(signal, variant) {
        this._connection.emit_signal(null, OVERLAY_OBJECT_PATH,
            OVERLAY_BUS_NAME, signal, variant);
    }
}

const loop = new GLib.MainLoop(null, false);
Gio.bus_own_name(
    Gio.BusType.SESSION,
    OVERLAY_BUS_NAME,
    Gio.BusNameOwnerFlags.NONE,
    connection => new OverlayService(connection),
    () => log(`lisa-overlayd: owning ${OVERLAY_BUS_NAME}`),
    () => {
        logError(new Error(`lost ${OVERLAY_BUS_NAME} (another instance running?)`));
        loop.quit();
    });
loop.run();
