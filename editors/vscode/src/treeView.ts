import * as path from "node:path";
import * as vscode from "vscode";
import type {
  CloneGroup,
  FallowCheckResult,
  FallowDupesResult,
  IssueCategory,
} from "./types.js";
import { ISSUE_CATEGORY_LABELS } from "./types.js";

/** Resolve a potentially relative CLI path to an absolute path. */
const resolveFilePath = (filePath: string): { absolute: string; relative: string } => {
  const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  const absolute = workspaceRoot && !path.isAbsolute(filePath)
    ? path.resolve(workspaceRoot, filePath)
    : filePath;
  const relative = workspaceRoot
    ? path.relative(workspaceRoot, absolute)
    : filePath;
  return { absolute, relative };
};

/** Icons per issue category. */
const CATEGORY_ICONS: Record<IssueCategory, string> = {
  "unused-files": "file-code",
  "unused-exports": "symbol-method",
  "unused-types": "symbol-interface",
  "unused-dependencies": "package",
  "unused-dev-dependencies": "package",
  "unused-enum-members": "symbol-enum-member",
  "unused-class-members": "symbol-field",
  "unresolved-imports": "error",
  "unlisted-dependencies": "package",
  "duplicate-exports": "files",
  "type-only-dependencies": "symbol-interface",
  "circular-dependencies": "sync",
};

/** Icons for individual issue items. */
const ISSUE_ICONS: Record<IssueCategory, string> = {
  "unused-files": "file",
  "unused-exports": "symbol-method",
  "unused-types": "symbol-interface",
  "unused-dependencies": "package",
  "unused-dev-dependencies": "package",
  "unused-enum-members": "symbol-enum-member",
  "unused-class-members": "symbol-field",
  "unresolved-imports": "error",
  "unlisted-dependencies": "package",
  "duplicate-exports": "copy",
  "type-only-dependencies": "package",
  "circular-dependencies": "sync",
};

type DeadCodeItem = CategoryItem | IssueItem;

class CategoryItem extends vscode.TreeItem {
  readonly issues: ReadonlyArray<IssueItem>;

  constructor(
    readonly category: IssueCategory,
    issues: ReadonlyArray<IssueItem>
  ) {
    super(
      `${ISSUE_CATEGORY_LABELS[category]} (${issues.length})`,
      vscode.TreeItemCollapsibleState.Collapsed
    );
    this.issues = issues;
    this.contextValue = "category";
    this.iconPath = new vscode.ThemeIcon(CATEGORY_ICONS[category] ?? "warning");
  }
}

class IssueItem extends vscode.TreeItem {
  constructor(
    label: string,
    readonly filePath: string,
    readonly line: number,
    readonly col: number,
    category: IssueCategory
  ) {
    super(label, vscode.TreeItemCollapsibleState.None);

    const { absolute, relative } = resolveFilePath(filePath);

    this.description = `${relative}:${line}`;
    this.tooltip = `${label}\n${absolute}:${line}:${col}`;
    this.contextValue = "issue";

    this.command = {
      command: "vscode.open",
      title: "Open File",
      arguments: [
        vscode.Uri.file(absolute),
        {
          selection: new vscode.Range(
            Math.max(0, line - 1),
            col,
            Math.max(0, line - 1),
            col
          ),
        },
      ],
    };

    this.iconPath = new vscode.ThemeIcon(ISSUE_ICONS[category] ?? "warning");
  }
}

export class DeadCodeTreeProvider
  implements vscode.TreeDataProvider<DeadCodeItem>
{
  private result: FallowCheckResult | null = null;
  private view: vscode.TreeView<DeadCodeItem> | null = null;

  private readonly _onDidChangeTreeData = new vscode.EventEmitter<
    DeadCodeItem | undefined | null | void
  >();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  setView(view: vscode.TreeView<DeadCodeItem>): void {
    this.view = view;
  }

  update(result: FallowCheckResult | null): void {
    this.result = result;
    this._onDidChangeTreeData.fire();
    this.updateBadge();
  }

  private updateBadge(): void {
    if (!this.view) {
      return;
    }
    if (!this.result) {
      this.view.badge = undefined;
      return;
    }
    const count =
      this.result.unused_files.length +
      this.result.unused_exports.length +
      this.result.unused_types.length +
      this.result.unused_dependencies.length +
      this.result.unused_dev_dependencies.length +
      this.result.unused_enum_members.length +
      this.result.unused_class_members.length +
      this.result.unresolved_imports.length +
      this.result.unlisted_dependencies.length +
      this.result.duplicate_exports.length +
      (this.result.type_only_dependencies?.length ?? 0) +
      (this.result.circular_dependencies?.length ?? 0);

    this.view.badge = count > 0
      ? { value: count, tooltip: `${count} issue${count === 1 ? "" : "s"}` }
      : undefined;
  }

  getTreeItem(element: DeadCodeItem): vscode.TreeItem {
    return element;
  }

  getChildren(element?: DeadCodeItem): DeadCodeItem[] {
    if (element instanceof CategoryItem) {
      return [...element.issues];
    }

    if (!this.result) {
      return [];
    }

    const categories: DeadCodeItem[] = [];

    const addCategory = (
      category: IssueCategory,
      items: ReadonlyArray<IssueItem>
    ): void => {
      if (items.length > 0) {
        categories.push(new CategoryItem(category, items));
      }
    };

    addCategory(
      "unused-files",
      this.result.unused_files.map(
        (f) => new IssueItem(path.basename(f.path), f.path, 1, 0, "unused-files")
      )
    );

    addCategory(
      "unused-exports",
      this.result.unused_exports.map(
        (e) => new IssueItem(e.export_name, e.path, e.line, e.col, "unused-exports")
      )
    );

    addCategory(
      "unused-types",
      this.result.unused_types.map(
        (e) => new IssueItem(e.export_name, e.path, e.line, e.col, "unused-types")
      )
    );

    addCategory(
      "unused-dependencies",
      this.result.unused_dependencies.map(
        (d) => new IssueItem(d.package_name, d.path, 1, 0, "unused-dependencies")
      )
    );

    addCategory(
      "unused-dev-dependencies",
      this.result.unused_dev_dependencies.map(
        (d) => new IssueItem(d.package_name, d.path, 1, 0, "unused-dev-dependencies")
      )
    );

    addCategory(
      "unused-enum-members",
      this.result.unused_enum_members.map(
        (m) =>
          new IssueItem(`${m.parent_name}.${m.member_name}`, m.path, m.line, m.col, "unused-enum-members")
      )
    );

    addCategory(
      "unused-class-members",
      this.result.unused_class_members.map(
        (m) =>
          new IssueItem(`${m.parent_name}.${m.member_name}`, m.path, m.line, m.col, "unused-class-members")
      )
    );

    addCategory(
      "unresolved-imports",
      this.result.unresolved_imports.map(
        (i) => new IssueItem(i.specifier, i.path, i.line, i.col, "unresolved-imports")
      )
    );

    addCategory(
      "unlisted-dependencies",
      this.result.unlisted_dependencies.map(
        (d) => new IssueItem(d.package_name, d.path, 1, 0, "unlisted-dependencies")
      )
    );

    addCategory(
      "duplicate-exports",
      this.result.duplicate_exports.flatMap((d) =>
        d.locations.map(
          (loc) => new IssueItem(d.export_name, loc.path, loc.line, loc.col, "duplicate-exports")
        )
      )
    );

    if (this.result.type_only_dependencies) {
      addCategory(
        "type-only-dependencies",
        this.result.type_only_dependencies.map(
          (d) => new IssueItem(d.package_name, d.path, 1, 0, "type-only-dependencies")
        )
      );
    }

    if (this.result.circular_dependencies) {
      addCategory(
        "circular-dependencies",
        this.result.circular_dependencies.map(
          (c) => new IssueItem(
            `${c.length} files`,
            c.files[0] ?? "",
            1,
            0,
            "circular-dependencies"
          )
        )
      );
    }

    return categories;
  }

  dispose(): void {
    this._onDidChangeTreeData.dispose();
  }
}

type DuplicateItem = CloneFamilyItem | CloneInstanceItem;

class CloneFamilyItem extends vscode.TreeItem {
  readonly instances: ReadonlyArray<CloneInstanceItem>;

  constructor(group: CloneGroup, index: number) {
    const instanceItems = group.instances.map(
      (inst) => new CloneInstanceItem(inst.file, inst.start_line, inst.end_line)
    );
    super(
      `Clone #${index + 1} (${group.line_count} lines, ${group.instances.length} instances)`,
      vscode.TreeItemCollapsibleState.Collapsed
    );
    this.instances = instanceItems;
    this.contextValue = "cloneFamily";
    this.iconPath = new vscode.ThemeIcon("files");
  }
}

class CloneInstanceItem extends vscode.TreeItem {
  constructor(
    readonly filePath: string,
    readonly startLine: number,
    readonly endLine: number
  ) {
    const basename = path.basename(filePath);
    super(
      `${basename}:${startLine}-${endLine}`,
      vscode.TreeItemCollapsibleState.None
    );

    const { absolute, relative } = resolveFilePath(filePath);

    this.description = relative;
    this.tooltip = `${absolute}:${startLine}-${endLine}`;
    this.contextValue = "cloneInstance";

    this.command = {
      command: "vscode.open",
      title: "Open File",
      arguments: [
        vscode.Uri.file(absolute),
        {
          selection: new vscode.Range(
            Math.max(0, startLine - 1),
            0,
            Math.max(0, endLine - 1),
            0
          ),
        },
      ],
    };

    this.iconPath = new vscode.ThemeIcon("copy");
  }
}

export class DuplicatesTreeProvider
  implements vscode.TreeDataProvider<DuplicateItem>
{
  private result: FallowDupesResult | null = null;

  private readonly _onDidChangeTreeData = new vscode.EventEmitter<
    DuplicateItem | undefined | null | void
  >();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  update(result: FallowDupesResult | null): void {
    this.result = result;
    this._onDidChangeTreeData.fire();
  }

  getTreeItem(element: DuplicateItem): vscode.TreeItem {
    return element;
  }

  getChildren(element?: DuplicateItem): DuplicateItem[] {
    if (element instanceof CloneFamilyItem) {
      return [...element.instances];
    }

    if (!this.result) {
      return [];
    }

    return this.result.clone_groups.map(
      (group, i) => new CloneFamilyItem(group, i)
    );
  }

  dispose(): void {
    this._onDidChangeTreeData.dispose();
  }
}
