export interface IssueTypeConfig {
  readonly "unused-files": boolean;
  readonly "unused-exports": boolean;
  readonly "unused-types": boolean;
  readonly "unused-dependencies": boolean;
  readonly "unused-dev-dependencies": boolean;
  readonly "unused-enum-members": boolean;
  readonly "unused-class-members": boolean;
  readonly "unresolved-imports": boolean;
  readonly "unlisted-dependencies": boolean;
  readonly "duplicate-exports": boolean;
}

export type DuplicationMode = "strict" | "mild" | "weak" | "semantic";

export type TraceLevel = "off" | "messages" | "verbose";

export interface FallowCheckResult {
  readonly unused_files: ReadonlyArray<UnusedFile>;
  readonly unused_exports: ReadonlyArray<UnusedExport>;
  readonly unused_types: ReadonlyArray<UnusedExport>;
  readonly unused_dependencies: ReadonlyArray<UnusedDependency>;
  readonly unused_dev_dependencies: ReadonlyArray<UnusedDependency>;
  readonly unused_enum_members: ReadonlyArray<UnusedMember>;
  readonly unused_class_members: ReadonlyArray<UnusedMember>;
  readonly unresolved_imports: ReadonlyArray<UnresolvedImport>;
  readonly unlisted_dependencies: ReadonlyArray<UnlistedDependency>;
  readonly duplicate_exports: ReadonlyArray<DuplicateExport>;
}

export interface UnusedFile {
  readonly path: string;
}

export interface UnusedExport {
  readonly path: string;
  readonly export_name: string;
  readonly line: number;
  readonly col: number;
}

export interface UnusedDependency {
  readonly package_name: string;
  readonly path: string;
}

export interface UnusedMember {
  readonly path: string;
  readonly parent_name: string;
  readonly member_name: string;
  readonly line: number;
  readonly col: number;
}

export interface UnresolvedImport {
  readonly path: string;
  readonly specifier: string;
  readonly line: number;
  readonly col: number;
}

export interface UnlistedDependency {
  readonly package_name: string;
  readonly path: string;
}

export interface DuplicateExport {
  readonly export_name: string;
  readonly locations: ReadonlyArray<string>;
}

export interface FallowDupesResult {
  readonly clone_groups: ReadonlyArray<CloneGroup>;
  readonly clone_families: ReadonlyArray<CloneFamily>;
  readonly stats: DupesStats;
}

export interface CloneGroup {
  readonly instances: ReadonlyArray<CloneInstance>;
  readonly token_count: number;
  readonly line_count: number;
}

export interface CloneInstance {
  readonly file: string;
  readonly start_line: number;
  readonly end_line: number;
  readonly start_col: number;
  readonly end_col: number;
  readonly fragment: string;
}

export interface CloneFamily {
  readonly files: ReadonlyArray<string>;
  readonly groups: ReadonlyArray<CloneGroup>;
  readonly total_duplicated_lines: number;
  readonly total_duplicated_tokens: number;
  readonly suggestions: ReadonlyArray<RefactoringSuggestion>;
}

export interface RefactoringSuggestion {
  readonly kind: "ExtractFunction" | "ExtractModule";
  readonly description: string;
  readonly estimated_savings: number;
}

export interface DupesStats {
  readonly total_files: number;
  readonly files_with_clones: number;
  readonly total_lines: number;
  readonly duplicated_lines: number;
  readonly total_tokens: number;
  readonly duplicated_tokens: number;
  readonly clone_groups: number;
  readonly clone_instances: number;
  readonly duplication_percentage: number;
}

export interface FallowFixResult {
  readonly dry_run: boolean;
  readonly fixes: ReadonlyArray<FixAction>;
  readonly total_fixed: number;
}

export interface FixAction {
  readonly type: string;
  readonly path?: string;
  readonly line?: number;
  readonly name?: string;
  readonly package?: string;
  readonly location?: string;
  readonly file?: string;
}

export type IssueCategory =
  | "unused-files"
  | "unused-exports"
  | "unused-types"
  | "unused-dependencies"
  | "unused-dev-dependencies"
  | "unused-enum-members"
  | "unused-class-members"
  | "unresolved-imports"
  | "unlisted-dependencies"
  | "duplicate-exports";

export const ISSUE_CATEGORY_LABELS: Record<IssueCategory, string> = {
  "unused-files": "Unused Files",
  "unused-exports": "Unused Exports",
  "unused-types": "Unused Types",
  "unused-dependencies": "Unused Dependencies",
  "unused-dev-dependencies": "Unused Dev Dependencies",
  "unused-enum-members": "Unused Enum Members",
  "unused-class-members": "Unused Class Members",
  "unresolved-imports": "Unresolved Imports",
  "unlisted-dependencies": "Unlisted Dependencies",
  "duplicate-exports": "Duplicate Exports",
};
