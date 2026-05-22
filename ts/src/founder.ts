export class FounderTree {
  private founders: Map<string, string>; // dir_path → participant_id
  private rootFounder: string;

  constructor(rootFounder: string) {
    this.founders = new Map();
    this.founders.set('.', rootFounder);
    this.rootFounder = rootFounder;
  }

  registerFromWrite(filePath: string, participant: string): void {
    // Extract directory from file path
    const lastSlash = filePath.lastIndexOf('/');
    if (lastSlash === -1) {
      return; // No directory (root level)
    }
    const dir = filePath.substring(0, lastSlash);

    // Walk through parent directories and register if not exists
    let current = '';
    for (const segment of dir.split('/')) {
      if (segment === '') continue;

      if (current !== '') {
        current += '/';
      }
      current += segment;

      if (!this.founders.has(current)) {
        this.founders.set(current, participant);
      }
    }
  }

  getFounder(dir: string): string | null {
    return this.founders.get(dir) || null;
  }

  isAncestorFounder(upperDir: string, lowerDir: string): boolean {
    if (upperDir === '.') {
      return lowerDir !== '.';
    }
    return lowerDir.startsWith(upperDir) && lowerDir !== upperDir;
  }

  listAll(): Array<[string, string]> {
    const entries = Array.from(this.founders.entries());
    entries.sort((a, b) => a[0].localeCompare(b[0]));
    return entries;
  }

  getRootFounder(): string {
    return this.rootFounder;
  }

  getFileFounder(filePath: string): string | null {
    const lastSlash = filePath.lastIndexOf('/');
    const dir = lastSlash === -1 ? '.' : filePath.substring(0, lastSlash);
    return this.getFounder(dir);
  }

  toJSON(): string {
    const obj: Record<string, string> = {};
    for (const [k, v] of this.founders.entries()) {
      obj[k] = v;
    }
    return JSON.stringify(obj);
  }

  static fromJSON(json: string, rootFounder: string): FounderTree {
    const obj = JSON.parse(json);
    const tree = new FounderTree(rootFounder);
    tree.founders = new Map(Object.entries(obj));
    return tree;
  }
}
