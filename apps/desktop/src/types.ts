export type RuleMode = "literal" | "regex";

export interface ReplacementRule {
  id: string;
  enabled: boolean;
  search: string;
  replacement: string;
  mode: RuleMode;
  caseSensitive: boolean;
  wholeWord: boolean;
  multiline: boolean;
  description: string;
}

export interface ScanRequest {
  targets: string[];
  filters: {
    includes: string[];
    excludes: string[];
    maxFileSize: number;
    recursive: boolean;
  };
  rules: ReplacementRule[];
  encodingOverrides: Record<string, string>;
  defaultEncodingOverride: string | null;
}

export interface DiffHunk {
  id: string;
  oldStart: number;
  oldLen: number;
  replacement: string;
  diff: string;
}

export interface FilePreview {
  path: string;
  sourceHash: string;
  resultHash: string;
  encoding: { name: string; hasBom: boolean; confidence: string } | null;
  lineEndings: string;
  matchCount: number;
  diff: string;
  hunks: DiffHunk[];
  changed: boolean;
  selected: boolean;
  warnings: string[];
  error: string | null;
}

export interface RulePreset {
  id: string;
  name: string;
  rules: ReplacementRule[];
}

export interface ScanResult {
  id: string;
  request: ScanRequest;
  files: FilePreview[];
  summary: { discovered: number; scanned: number; changed: number; matches: number; skipped: number; failed: number };
  createdAt: string;
}

export interface OperationManifest {
  schemaVersion: number;
  id: string;
  scanId: string;
  createdAt: string;
  files: Array<{
    path: string;
    sourceHash: string;
    resultHash: string;
    backupFile: string;
    status: "changed" | "restored" | "skipped" | "conflict" | "failed";
    message: string | null;
  }>;
}

export const DEFAULT_EXCLUDES = [
  "**/.git/**", "**/.svn/**", "**/.hg/**", "**/node_modules/**",
  "**/target/**", "**/dist/**", "**/build/**", "**/.speedreplace/**",
];

export function newRule(): ReplacementRule {
  return {
    id: crypto.randomUUID(), enabled: true, search: "", replacement: "",
    mode: "literal", caseSensitive: true, wholeWord: false, multiline: true, description: "",
  };
}
