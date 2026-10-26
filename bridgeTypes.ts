/* ************************************************************************** */
/*                                                                            */
/*                                                        :::      ::::::::   */
/*   bridgeTypes.ts                                     :+:      :+:    :+:   */
/*                                                    +:+ +:+         +:+     */
/*   By: dlesieur <dlesieur@student.42.fr>          +#+  +:+       +#+        */
/*                                                +#+#+#+#+#+   +#+           */
/*   Created: 2026/05/18 21:19:17 by dlesieur          #+#    #+#             */
/*   Updated: 2026/05/18 21:19:17 by dlesieur         ###   ########.fr       */
/*                                                                            */
/* ************************************************************************** */

/** @file bridgeTypes.ts — Shared types + serialization helpers for the WASM bridge. */

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
  /** Stable machine-readable code of a runtime failure (e.g. "budget_exceeded").
   *  Absent when the engine could not classify the failure. */
  error_code?: string;
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

export function toJsonValue(val: unknown): FormulaValue {
  if (val === null || val === undefined) return { type: 'empty' };
  if (typeof val === 'number') return { type: 'number', value: val };
  if (typeof val === 'string') return { type: 'text', value: val };
  if (typeof val === 'boolean') return { type: 'boolean', value: val };
  if (val instanceof Date) return { type: 'date', value: val.getTime() };
  if (Array.isArray(val)) return { type: 'array', value: val.map(toJsonValue) };
  const range = val as { start?: unknown; end?: unknown };
  if (range.start instanceof Date && range.end instanceof Date) {
    return { type: 'dateRange', value: { start: range.start.getTime(), end: range.end.getTime() } };
  }
  return { type: 'text', value: String(val) };
}

/** Decodes an engine value. Dates arrive as epoch ms and arrays as nested
 *  FormulaValues; passed through raw they rendered as `1792973683132` and
 *  `[object Object]`. */
export function fromFormulaValue(fv: FormulaValue | undefined): unknown {
  if (!fv || fv.type === 'empty') return '';
  if (fv.type === 'date' && typeof fv.value === 'number') return new Date(fv.value);
  if (fv.type === 'dateRange' && fv.value && typeof fv.value === 'object') {
    const { start, end } = fv.value as { start?: number; end?: number };
    return { start: new Date(start ?? Number.NaN), end: new Date(end ?? Number.NaN) };
  }
  if (fv.type === 'array' && Array.isArray(fv.value)) {
    return (fv.value as FormulaValue[]).map(fromFormulaValue);
  }
  return fv.value ?? '';
}

export function propsToJson(props: PropertyMap): string {
  return JSON.stringify(props);
}
