// WASM bridge — types and value conversion helpers

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

export function toJsonValue(val: unknown): FormulaValue {
  if (val === null || val === undefined) return { type: 'empty' };
  if (typeof val === 'number') return { type: 'number', value: val };
  if (typeof val === 'string') return { type: 'text', value: val };
  if (typeof val === 'boolean') return { type: 'boolean', value: val };
  if (val instanceof Date) return { type: 'date', value: val.getTime() };
  if (Array.isArray(val)) return { type: 'array', value: val.map(toJsonValue) };
  if (typeof val === 'object' && val !== null && 'start' in val && 'end' in val) {
    const obj = val as { start: unknown; end: unknown };
    const startMs = obj.start instanceof Date ? obj.start.getTime() : Number(obj.start);
    const endMs = obj.end instanceof Date ? obj.end.getTime() : Number(obj.end);
    return { type: 'dateRange', value: { start: startMs, end: endMs } };
  }
  return { type: 'text', value: String(val) };
}

export function fromFormulaValue(fv: FormulaValue | undefined): unknown {
  if (!fv || fv.type === 'empty') return '';
  switch (fv.type) {
    case 'number': return fv.value;
    case 'text': return fv.value;
    case 'boolean': return fv.value;
    case 'date': return typeof fv.value === 'number' ? new Date(fv.value).toISOString() : '';
    case 'dateRange': {
      const v = fv.value as { start: number; end: number };
      return `${new Date(v.start).toISOString()} → ${new Date(v.end).toISOString()}`;
    }
    case 'array': return (fv.value as FormulaValue[]).map(fromFormulaValue);
    default: return '';
  }
}

export function propsToJson(props: PropertyMap): string {
  const converted: Record<string, FormulaValue> = {};
  for (const [key, val] of Object.entries(props)) {
    converted[key] = toJsonValue(val);
  }
  return JSON.stringify(converted);
}
