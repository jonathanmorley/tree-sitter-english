/* tslint:disable */
/* eslint-disable */

/**
 * Analyze `text`, returning a JSON string (never throws across the
 * boundary; errors serialize as `{"error": ...}`).
 */
export function analyze(text: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly analyze: (a: number, b: number) => [number, number];
    readonly tree_sitter_english_external_scanner_create: () => number;
    readonly tree_sitter_english_external_scanner_deserialize: (a: number, b: number, c: number) => void;
    readonly tree_sitter_english_external_scanner_destroy: (a: number) => void;
    readonly tree_sitter_english_external_scanner_scan: (a: number, b: number, c: number) => number;
    readonly tree_sitter_english_external_scanner_serialize: (a: number, b: number) => number;
    readonly abort: () => void;
    readonly calloc: (a: number, b: number) => number;
    readonly free: (a: number) => void;
    readonly malloc: (a: number) => number;
    readonly realloc: (a: number, b: number) => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
