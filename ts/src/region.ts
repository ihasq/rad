import type { CodeRegion } from './types';

export class RegionMap {
  private regions: CodeRegion[] = [];

  register(region: CodeRegion): boolean {
    if (this.getOwner(region.filePath, region.startLine) !== null) {
      return false;
    }
    this.regions.push(region);
    return true;
  }

  getOwner(file: string, line: number): string | null {
    for (const r of this.regions) {
      if (r.filePath === file && line >= r.startLine && line <= r.endLine) {
        return r.ownerId;
      }
    }
    return null;
  }

  list(file: string): CodeRegion[] {
    return this.regions.filter(r => r.filePath === file);
  }

  getRole(file: string, line: number, participant: string): string {
    const owner = this.getOwner(file, line);
    if (owner === null) return 'unowned';
    return owner === participant ? 'leader' : 'follower';
  }

  getOwnerByRegionId(regionId: string): string | undefined {
    const region = this.regions.find(r => r.id === regionId);
    return region?.ownerId;
  }

  getById(regionId: string): CodeRegion | undefined {
    return this.regions.find(r => r.id === regionId);
  }
}
