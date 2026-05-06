/* tslint:disable */
/* eslint-disable */

/**
 * Batch evaluate a formula over multiple row contexts → returns JSON { ok, values?, error? }
 */
export function batch_evaluate(handle: number, rows_json: string): string;

/**
 * Compile a formula string → returns JSON { ok, handle?, error? }
 */
export function compile(formula: string): string;

/**
 * One-shot compile + evaluate (convenience for single evaluations)
 */
export function eval_formula(formula: string, props_json: string): string;

/**
 * Evaluate a compiled formula with given properties JSON → returns JSON { ok, value?, error? }
 */
export function evaluate(handle: number, props_json: string): string;

/**
 * Free a compiled formula from cache
 */
export function free_formula(handle: number): void;

/**
 * Get dependencies of a compiled formula → JSON string array
 */
export function get_dependencies(handle: number): string;

/**
 * Validate a formula without executing → returns JSON { ok, errors[], dependencies[] }
 */
export function validate(formula: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly compile: (a: number, b: number, c: number) => void;
    readonly evaluate: (a: number, b: number, c: number, d: number) => void;
    readonly batch_evaluate: (a: number, b: number, c: number, d: number) => void;
    readonly validate: (a: number, b: number, c: number) => void;
    readonly get_dependencies: (a: number, b: number) => void;
    readonly free_formula: (a: number) => void;
    readonly eval_formula: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
    readonly __wbindgen_export: (a: number, b: number) => number;
    readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_export3: (a: number, b: number, c: number) => void;
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
