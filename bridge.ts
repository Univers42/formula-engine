/* ************************************************************************** */
/*                                                                            */
/*                                                        :::      ::::::::   */
/*   bridge.ts                                          :+:      :+:    :+:   */
/*                                                    +:+ +:+         +:+     */
/*   By: dlesieur <dlesieur@student.42.fr>          +#+  +:+       +#+        */
/*                                                +#+#+#+#+#+   +#+           */
/*   Created: 2026/05/18 21:19:17 by dlesieur          #+#    #+#             */
/*   Updated: 2026/05/18 21:19:17 by dlesieur         ###   ########.fr       */
/*                                                                            */
/* ************************************************************************** */

/**
 * @file bridge.ts — TypeScript ↔ WASM bridge for the Rust formula engine.
 *
 * Loads the compiled WASM module from the co-located `pkg/` directory and
 * exposes a high-level, ergonomic API that the rest of the app can call
 * without knowing anything about serialization or handles.
 *
 * Graceful degradation: if WASM fails to load (missing binary, unsupported
 * browser), every function returns a safe default and the app continues to
 * work — formula columns simply show blank values.
 */

import type {
  CompileResult, EvalResult, BatchResult, ValidateResult, PropertyMap, FormulaValue,
} from './bridgeTypes';
import { toJsonValue, fromFormulaValue } from './bridgeTypes';
import { desugarFormula } from '../formula/desugarFormula';

export type {
  FormulaValue, CompileResult, EvalResult, BatchResult,
  ValidateResult, PropertyMap,
} from './bridgeTypes';

/* ── WASM module type (matches wasm-bindgen glue exports) ───────────────── */

interface WasmEngine {
  compile(formula: string): string;
  evaluate(handle: number, propsJson: string): string;
  batch_evaluate(handle: number, rowsJson: string): string;
  eval_formula(formula: string, propsJson: string): string;
  validate(formula: string): string;
  get_dependencies(handle: number): string;
  free_formula(handle: number): void;
}

/* ── Singleton state ────────────────────────────────────────────────────── */

let wasmEngine: WasmEngine | null = null;
let initPromise: Promise<boolean> | null = null;
let initFailed = false;

const formulaHandleCache = new Map<string, number>();

declare const __OBJECT_DATABASE_DISABLE_WASM__: boolean | undefined;

/** Stable placeholder returned when the optional WASM formula engine is unavailable. */
export const FORMULA_ENGINE_UNAVAILABLE = '#ENGINE_UNAVAILABLE';

/* ── Initialization (lazy, singleton, deduped) ──────────────────────────── */

export async function initFormulaEngine(): Promise<boolean> {
  if (wasmEngine) return true;
  if (initFailed) return false;
  if (isFormulaEngineDisabled()) {
    initFailed = true;
    return false;
  }

  initPromise ??= (async () => {
    try {
      const mod = await import('./pkg/formula_engine.js');
      await mod.default();                   // WebAssembly.instantiate
      wasmEngine = mod as unknown as WasmEngine;
      return true;
    } catch (err) {
      console.warn('[formula-engine] WASM init failed, formulas disabled:', err);
      initFailed = true;
      return false;
    }
  })();

  return initPromise;
}

function isFormulaEngineDisabled(): boolean {
  return __OBJECT_DATABASE_DISABLE_WASM__ !== undefined && __OBJECT_DATABASE_DISABLE_WASM__;
}

export function isWasmReady(): boolean {
  return wasmEngine !== null;
}

/* ── Compile (with handle cache) ────────────────────────────────────────── */

export function compileFormula(formula: string): CompileResult | null {
  if (!wasmEngine) return null;
  formula = desugarFormula(formula);

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

/* ── One-shot eval (compile + evaluate in one WASM call) ────────────────── */

export function evalFormula(formula: string, props: PropertyMap): unknown {
  if (!wasmEngine) return FORMULA_ENGINE_UNAVAILABLE;
  try {
    const propsJson = serializeProps(props);
    const resultJson = wasmEngine.eval_formula(desugarFormula(formula), propsJson);
    const result: EvalResult = JSON.parse(resultJson);
    return result.ok ? fromFormulaValue(result.value) : FORMULA_ENGINE_UNAVAILABLE;
  } catch {
    return FORMULA_ENGINE_UNAVAILABLE;
  }
}

/* ── Evaluate a pre-compiled handle ─────────────────────────────────────── */

export function evaluateHandle(handle: number, props: PropertyMap): unknown {
  if (!wasmEngine) return FORMULA_ENGINE_UNAVAILABLE;
  try {
    const propsJson = serializeProps(props);
    const resultJson = wasmEngine.evaluate(handle, propsJson);
    const result: EvalResult = JSON.parse(resultJson);
    return result.ok ? fromFormulaValue(result.value) : FORMULA_ENGINE_UNAVAILABLE;
  } catch {
    return FORMULA_ENGINE_UNAVAILABLE;
  }
}

/* ── Batch evaluation (single WASM round-trip for N rows) ───────────────── */

export function batchEvaluate(formula: string, rows: PropertyMap[]): unknown[] {
  if (!wasmEngine) return rows.map(() => FORMULA_ENGINE_UNAVAILABLE);
  try {
    // compileFormula desugars; caching there keys on the normalized form.
    const compiled = compileFormula(formula);
    if (!compiled?.ok || compiled.handle === undefined) return rows.map(() => FORMULA_ENGINE_UNAVAILABLE);

    const rowsJson = JSON.stringify(
      rows.map(r => {
        const converted: Record<string, FormulaValue> = {};
        for (const [key, val] of Object.entries(r)) {
          converted[key] = toJsonValue(val);
        }
        return converted;
      }),
    );

    const resultJson = wasmEngine.batch_evaluate(compiled.handle, rowsJson);
    const result: BatchResult = JSON.parse(resultJson);
    return result.ok && result.values
      ? result.values.map(fromFormulaValue)
      : rows.map(() => FORMULA_ENGINE_UNAVAILABLE);
  } catch {
    return rows.map(() => FORMULA_ENGINE_UNAVAILABLE);
  }
}

/* ── Validation (parse-only, no execution) ──────────────────────────────── */

export function validateFormula(formula: string): ValidateResult {
  if (!wasmEngine) return { ok: true, errors: [], dependencies: [] };
  try {
    const resultJson = wasmEngine.validate(desugarFormula(formula));
    return JSON.parse(resultJson);
  } catch {
    return { ok: true, errors: [], dependencies: [] };
  }
}

/* ── Dependencies ───────────────────────────────────────────────────────── */

export function getDependencies(handle: number): string[] {
  if (!wasmEngine) return [];
  try {
    const resultJson = wasmEngine.get_dependencies(handle);
    return JSON.parse(resultJson);
  } catch {
    return [];
  }
}

/* ── Handle lifecycle ───────────────────────────────────────────────────── */

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

export function clearFormulaCache(): void {
  formulaHandleCache.clear();
}

/* ── Internal helpers ───────────────────────────────────────────────────── */

function serializeProps(props: PropertyMap): string {
  const converted: Record<string, FormulaValue> = {};
  for (const [key, val] of Object.entries(props)) {
    converted[key] = toJsonValue(val);
  }
  return JSON.stringify(converted);
}
