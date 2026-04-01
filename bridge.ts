// WASM formula engine — TypeScript bridge
import {
  FormulaValue, CompileResult, EvalResult, BatchResult,
  ValidateResult, PropertyMap, toJsonValue, fromFormulaValue, propsToJson,
} from './bridgeTypes';
export type { FormulaValue, CompileResult, EvalResult, BatchResult, ValidateResult, PropertyMap } from './bridgeTypes';

interface WasmEngine {
  compile(formula: string): string;
  evaluate(handle: number, propsJson: string): string;
  batch_evaluate(handle: number, rowsJson: string): string;
  validate(formula: string): string;
  get_dependencies(handle: number): string;
  free_formula(handle: number): void;
  eval_formula(formula: string, propsJson: string): string;
}

let wasmEngine: WasmEngine | null = null;
let initPromise: Promise<void> | null = null;
let initFailed = false;

// Compiled formula handle cache: formula string → handle ID
const formulaHandleCache = new Map<string, number>();

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

/**
 * Compile a formula and return a reusable handle.
 * Returns null if compilation fails.
 */
export function compileFormula(formula: string): CompileResult | null {
  if (!wasmEngine) return null;
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
 * Batch evaluate a formula over many rows at once.
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

/** Validate a formula (check syntax, return dependencies). */
export function validateFormula(formula: string): ValidateResult {
  if (!wasmEngine) {
    try {
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

/** Get dependencies (property names) of a compiled formula. */
export function getDependencies(handle: number): string[] {
  if (!wasmEngine) return [];
  const json = wasmEngine.get_dependencies(handle);
  return JSON.parse(json);
}

/** Free a compiled formula from the WASM cache. */
export function freeFormula(handle: number): void {
  if (!wasmEngine) return;
  wasmEngine.free_formula(handle);
  for (const [formula, h] of formulaHandleCache) {
    if (h === handle) {
      formulaHandleCache.delete(formula);
      break;
    }
  }
}

/** Clear all cached compiled formulas. */
export function clearFormulaCache(): void {
  if (wasmEngine) {
    for (const handle of formulaHandleCache.values()) {
      wasmEngine.free_formula(handle);
    }
  }
  formulaHandleCache.clear();
}

// Start loading WASM immediately on import (non-blocking)
initFormulaEngine().catch(() => {});
