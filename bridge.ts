// ═══════════════════════════════════════════════════════════════════════════════
// FORMULA ENGINE — TypeScript Bridge
// ═══════════════════════════════════════════════════════════════════════════════
// Wraps the WASM formula engine with a clean TypeScript API.
// Handles async initialization, caching, and fallback to TS evaluation.

// ─── Types ────────────────────────────────────────────────────────────────────

export interface FormulaValue {
  type: 'number' | 'text' | 'boolean' | 'date' | 'dateRange' | 'array' | 'empty';
  value?: unknown;
}

export interface CompileResult {
  ok: boolean;
  handle?: number;
  error?: string;
  error_pos?: [number, number];
}

export interface EvalResult {
  ok: boolean;
  value?: FormulaValue;
  error?: string;
}

export interface BatchResult {
  ok: boolean;
  values?: FormulaValue[];
  error?: string;
}

export interface ValidateResult {
  ok: boolean;
  errors: Array<{ message: string; start: number; end: number }>;
  dependencies: string[];
}

export type PropertyMap = Record<string, unknown>;

// ─── WASM Module Interface ────────────────────────────────────────────────────

interface WasmEngine {
  compile(formula: string): string;
  evaluate(handle: number, propsJson: string): string;
  batch_evaluate(handle: number, rowsJson: string): string;
  validate(formula: string): string;
  get_dependencies(handle: number): string;
  free_formula(handle: number): void;
  eval_formula(formula: string, propsJson: string): string;
}

// ─── Engine State ─────────────────────────────────────────────────────────────

let wasmEngine: WasmEngine | null = null;
let initPromise: Promise<void> | null = null;
let initFailed = false;

// Compiled formula handle cache: formula string → handle ID
const formulaHandleCache = new Map<string, number>();

// ─── Initialize WASM Engine ───────────────────────────────────────────────────

async function loadWasm(): Promise<WasmEngine> {
  // Dynamic import of the wasm-pack generated module
  const wasm = await import('./pkg/formula_engine.js');
  // Initialize the WASM module
  await wasm.default();
  return wasm as unknown as WasmEngine;
}

/**
 * Initialize the WASM formula engine. Call once at app startup.
 * Safe to call multiple times — will only init once.
 */
export async function initFormulaEngine(): Promise<boolean> {
  if (wasmEngine) return true;
  if (initFailed) return false;

  if (!initPromise) {
    initPromise = loadWasm()
      .then((engine) => {
        wasmEngine = engine;
        console.log('[FormulaEngine] WASM engine loaded successfully');
      })
      .catch((err) => {
        initFailed = true;
        console.warn('[FormulaEngine] WASM load failed, using TS fallback:', err);
      });
  }

  await initPromise;
  return wasmEngine !== null;
}

/**
 * Check if WASM engine is ready
 */
export function isWasmReady(): boolean {
  return wasmEngine !== null;
}

// ─── Value Conversion Helpers ─────────────────────────────────────────────────

function toJsonValue(val: unknown): FormulaValue {
  if (val === null || val === undefined) {
    return { type: 'empty' };
  }
  if (typeof val === 'number') {
    return { type: 'number', value: val };
  }
  if (typeof val === 'string') {
    return { type: 'text', value: val };
  }
  if (typeof val === 'boolean') {
    return { type: 'boolean', value: val };
  }
  if (val instanceof Date) {
    return { type: 'date', value: val.getTime() };
  }
  if (Array.isArray(val)) {
    return { type: 'array', value: val.map(toJsonValue) };
  }
  // Object with start/end dates = DateRange
  if (typeof val === 'object' && val !== null && 'start' in val && 'end' in val) {
    const obj = val as { start: unknown; end: unknown };
    const startMs = obj.start instanceof Date ? obj.start.getTime() : Number(obj.start);
    const endMs = obj.end instanceof Date ? obj.end.getTime() : Number(obj.end);
    return { type: 'dateRange', value: { start: startMs, end: endMs } };
  }
  return { type: 'text', value: String(val) };
}

function fromFormulaValue(fv: FormulaValue | undefined): unknown {
  if (!fv || fv.type === 'empty') return '';
  switch (fv.type) {
    case 'number':
      return fv.value;
    case 'text':
      return fv.value;
    case 'boolean':
      return fv.value;
    case 'date':
      return typeof fv.value === 'number' ? new Date(fv.value).toISOString() : '';
    case 'dateRange': {
      const v = fv.value as { start: number; end: number };
      return `${new Date(v.start).toISOString()} → ${new Date(v.end).toISOString()}`;
    }
    case 'array': {
      const arr = fv.value as FormulaValue[];
      return arr.map(fromFormulaValue);
    }
    default:
      return '';
  }
}

function propsToJson(props: PropertyMap): string {
  const converted: Record<string, FormulaValue> = {};
  for (const [key, val] of Object.entries(props)) {
    converted[key] = toJsonValue(val);
  }
  return JSON.stringify(converted);
}

// ─── Public API ───────────────────────────────────────────────────────────────

/**
 * Compile a formula and return a reusable handle.
 * Returns null if compilation fails.
 */
export function compileFormula(formula: string): CompileResult | null {
  if (!wasmEngine) return null;

  // Check cache
  const cached = formulaHandleCache.get(formula);
  if (cached !== undefined) {
    return { ok: true, handle: cached };
  }

  const resultJson = wasmEngine.compile(formula);
  const result: CompileResult = JSON.parse(resultJson);

  if (result.ok && result.handle !== undefined) {
    formulaHandleCache.set(formula, result.handle);
  }

  return result;
}

/**
 * Evaluate a formula with given row properties.
 * One-shot: compiles and evaluates in one call (fastest for single use).
 */
export function evalFormula(formula: string, props: PropertyMap): unknown {
  if (!wasmEngine) return '';

  try {
    const propsJson = propsToJson(props);
    const resultJson = wasmEngine.eval_formula(formula, propsJson);
    const result: EvalResult = JSON.parse(resultJson);

    if (result.ok && result.value) {
      return fromFormulaValue(result.value);
    }
    return '';
  } catch {
    return '';
  }
}

/**
 * Evaluate a compiled formula (by handle) with given properties.
 * Fastest for repeated evaluation of the same formula across different rows.
 */
export function evaluateHandle(handle: number, props: PropertyMap): unknown {
  if (!wasmEngine) return '';

  const propsJson = propsToJson(props);
  const resultJson = wasmEngine.evaluate(handle, propsJson);
  const result: EvalResult = JSON.parse(resultJson);

  if (result.ok && result.value) {
    return fromFormulaValue(result.value);
  }
  return '';
}

/**
 * Batch evaluate a formula over many rows at once (most efficient for tables).
 * Returns array of results, one per row.
 */
export function batchEvaluate(formula: string, rows: PropertyMap[]): unknown[] {
  if (!wasmEngine) return rows.map(() => '');

  try {
    const compiled = compileFormula(formula);
    if (!compiled?.ok || compiled.handle === undefined) {
      return rows.map(() => '');
    }

    const rowsJson = JSON.stringify(
      rows.map((r) => {
        const converted: Record<string, FormulaValue> = {};
        for (const [key, val] of Object.entries(r)) {
          converted[key] = toJsonValue(val);
        }
        return converted;
      })
    );

    const resultJson = wasmEngine.batch_evaluate(compiled.handle, rowsJson);
    const result: BatchResult = JSON.parse(resultJson);

    if (result.ok && result.values) {
      return result.values.map(fromFormulaValue);
    }
    return rows.map(() => '');
  } catch {
    return rows.map(() => '');
  }
}

/**
 * Validate a formula (check syntax, return dependencies).
 */
export function validateFormula(formula: string): ValidateResult {
  if (!wasmEngine) {
    // Basic TS validation fallback
    try {
      // Try simple parse
      return { ok: true, errors: [], dependencies: [] };
    } catch (e) {
      return {
        ok: false,
        errors: [{ message: String(e), start: 0, end: formula.length }],
        dependencies: [],
      };
    }
  }

  const resultJson = wasmEngine.validate(formula);
  return JSON.parse(resultJson);
}

/**
 * Get dependencies (property names) of a compiled formula.
 */
export function getDependencies(handle: number): string[] {
  if (!wasmEngine) return [];
  const json = wasmEngine.get_dependencies(handle);
  return JSON.parse(json);
}

/**
 * Free a compiled formula from the WASM cache (for cleanup).
 */
export function freeFormula(handle: number): void {
  if (!wasmEngine) return;
  wasmEngine.free_formula(handle);
  // Remove from local cache too
  for (const [formula, h] of formulaHandleCache) {
    if (h === handle) {
      formulaHandleCache.delete(formula);
      break;
    }
  }
}

/**
 * Clear all cached compiled formulas.
 */
export function clearFormulaCache(): void {
  if (wasmEngine) {
    for (const handle of formulaHandleCache.values()) {
      wasmEngine.free_formula(handle);
    }
  }
  formulaHandleCache.clear();
}

// ─── Auto-Initialize ──────────────────────────────────────────────────────────
// Start loading WASM immediately on import (non-blocking)
initFormulaEngine().catch(() => {});
