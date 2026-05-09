// Centralizes the contract: `name_en` (or `label_en`) holds Latin/transliterated
// text, `name_ar` (or `label`) holds Arabic. The backend sometimes emits the same
// Arabic string in both fields when no transliteration exists; this module fixes
// that at the API boundary so consumers don't need per-field guards.

const ARABIC_RE = /[؀-ۿݐ-ݿࢠ-ࣿﭐ-﷿ﹰ-﻿]/;

export function isArabicScript(s: string | null | undefined): boolean {
  return !!s && ARABIC_RE.test(s);
}

interface BilingualName {
  name_en?: string | null;
  name_ar?: string | null;
}

export function normalizeBilingualName<T extends BilingualName>(record: T): T {
  const en = record.name_en?.trim() || null;
  const ar = record.name_ar?.trim() || null;
  if (en && (en === ar || isArabicScript(en))) {
    record.name_ar = ar ?? en;
    record.name_en = null;
  } else {
    record.name_en = en;
    record.name_ar = ar;
  }
  return record;
}

export function normalizeBilingualNames<T extends BilingualName>(records: T[] | null | undefined): T[] {
  if (!records) return [];
  for (const r of records) normalizeBilingualName(r);
  return records;
}

interface GraphLikeNode {
  data: { label?: string | null; label_en?: string | null };
}

export function normalizeGraphLabels<T extends { nodes: GraphLikeNode[] }>(data: T): T {
  for (const node of data.nodes) {
    const en = node.data.label_en?.trim() || null;
    const ar = node.data.label?.trim() || null;
    if (en && (en === ar || isArabicScript(en))) {
      node.data.label = ar ?? en;
      node.data.label_en = null;
    } else {
      node.data.label_en = en;
      node.data.label = ar;
    }
  }
  return data;
}

export function bilingualDisplayName(
  record: BilingualName | null | undefined,
  lang: 'en' | 'ar',
  fallback = '',
): string {
  if (!record) return fallback;
  if (lang === 'en') return record.name_en || record.name_ar || fallback;
  return record.name_ar || record.name_en || fallback;
}

export function bilingualIsArabic(
  record: BilingualName | null | undefined,
  lang: 'en' | 'ar',
): boolean {
  if (!record) return lang === 'ar';
  if (lang === 'ar') return true;
  return !record.name_en;
}
