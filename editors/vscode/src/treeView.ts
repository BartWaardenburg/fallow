import * as path from "node:path";
import * as vscode from "vscode";
import type {
  CloneGroup,
  FallowCheckResult,
  FallowDupesResult,
  IssueCategory,
} from "./types.js";
import { ISSUE_CATEGORY_LABELS } from "./types.js";

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
  }
}

class IssueItem extends vscode.TreeItem {
  constructor(
    label: string,
    readonly filePath: string,
    readonly line: number,
    readonly col: number
  ) {
    super(label, vscode.TreeItemCollapsibleState.None);

    const relativePath = vscode.workspace.workspaceFolders?.[0]
      ? path.relative(vscode.workspace.workspaceFolders[0].uri.fsPath, filePath)
      : filePath;

    this.description = `${relativePath}:${line}`;
    this.tooltip = `${label}\n${filePath}:${line}:${col}`;
    this.contextValue = "issue";

    this.command = {
      command: "vscode.open",
      title: "Open File",
      arguments: [
        vscode.Uri.file(filePath),
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

    this.iconPath = new vscode.ThemeIcon("warning");
  }
}

export class DeadCodeTreeProvider
  implements vscode.TreeDataProvider<DeadCodeItem>
{
  private result: FallowCheckResult | null = null;

  private readonly _onDidChangeTreeData = new vscode.EventEmitter<
    DeadCodeItem | undefined | null | void
  >();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  update(result: FallowCheckResult | null): void {
    this.result = result;
    this._onDidChangeTreeData.fire();
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
        (f) => new IssueItem(path.basename(f.path), f.path, 1, 0)
      )
    );

    addCategory(
      "unused-exports",
      this.result.unused_exports.map(
        (e) => new IssueItem(e.export_name, e.path, e.line, e.col)
      )
    );

    addCategory(
      "unused-types",
      this.result.unused_types.map(
        (e) => new IssueItem(e.export_name, e.path, e.line, e.col)
      )
    );

    addCategory(
      "unused-dependencies",
      this.result.unused_dependencies.map(
        (d) => new IssueItem(d.package_name, d.path, 1, 0)
      )
    );

    addCategory(
      "unused-dev-dependencies",
      this.result.unused_dev_dependencies.map(
        (d) => new IssueItem(d.package_name, d.path, 1, 0)
      )
    );

    addCategory(
      "unused-enum-members",
      this.result.unused_enum_members.map(
        (m) =>
          new IssueItem(`${m.parent_name}.${m.member_name}`, m.path, m.line, m.col)
      )
    );

    addCategory(
      "unused-class-members",
      this.result.unused_class_members.map(
        (m) =>
          new IssueItem(`${m.parent_name}.${m.member_name}`, m.path, m.line, m.col)
      )
    );

    addCategory(
      "unresolved-imports",
      this.result.unresolved_imports.map(
        (i) => new IssueItem(i.specifier, i.path, i.line, i.col)
      )
    );

    addCategory(
      "unlisted-dependencies",
      this.result.unlisted_dependencies.map(
        (d) => new IssueItem(d.package_name, d.path, 1, 0)
      )
    );

    addCategory(
      "duplicate-exports",
      this.result.duplicate_exports.flatMap((d) =>
        d.locations.map(
          (loc) => new IssueItem(d.export_name, loc, 1, 0)
        )
      )
    );

    if (this.result.type_only_dependencies) {
      addCategory(
        "type-only-dependencies",
        this.result.type_only_dependencies.map(
          (d) => new IssueItem(d.package_name, d.path, 1, 0)
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
            0
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

    const relativePath = vscode.workspace.workspaceFolders?.[0]
      ? path.relative(vscode.workspace.workspaceFolders[0].uri.fsPath, filePath)
      : filePath;

    this.description = relativePath;
    this.tooltip = `${filePath}:${startLine}-${endLine}`;
    this.contextValue = "cloneInstance";

    this.command = {
      command: "vscode.open",
      title: "Open File",
      arguments: [
        vscode.Uri.file(filePath),
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
