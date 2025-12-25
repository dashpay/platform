const semver = require('semver');

/**
 * Special workspaces that have a major version offset from the root.
 * These packages lead the root major version by a fixed amount.
 */
const SPECIAL_WORKSPACE_OFFSETS = {
  '@dashevo/dash-spv': 1,
  'dash': 3,
  '@dashevo/wallet-lib': 7,
};

/**
 * Given a list of version ranges, pick the highest one deterministically.
 * For semver-valid ranges, picks the highest minVersion.
 * Falls back to alphabetical sort for non-semver ranges.
 */
function pickHighestRange(ranges) {
  const sorted = [...ranges].sort((a, b) => {
    const minA = semver.minVersion(a);
    const minB = semver.minVersion(b);

    // Both are valid semver ranges - compare by minVersion
    if (minA && minB) {
      const cmp = semver.compare(minB, minA); // descending
      if (cmp !== 0) return cmp;
    }

    // If one is valid and other isn't, prefer the valid one
    if (minA && !minB) return -1;
    if (!minA && minB) return 1;

    // Both invalid or equal - sort alphabetically for stability
    return a.localeCompare(b);
  });

  return sorted[0];
}

/**
 * Apply a major version offset to a version string.
 * @param {string} rootVersion - The root version string (e.g., "1.2.3" or "1.2.3-dev.1")
 * @param {number} offset - The major version offset to apply
 * @returns {string} - The new version with offset applied
 */
function applyVersionOffset(rootVersion, offset) {
  const match = rootVersion.match(/^(\d+)(.*)$/);
  if (!match) {
    throw new Error(`Invalid version format: ${rootVersion}`);
  }
  const major = parseInt(match[1], 10);
  const rest = match[2];
  return `${major + offset}${rest}`;
}

module.exports = {
  constraints: async ({ Yarn }) => {
    // Get the root workspace version for version enforcement
    const rootWorkspace = Yarn.workspace({ ident: '@dashevo/platform' });
    const rootVersion = rootWorkspace?.manifest?.version;

    // Prevent two workspaces from depending on conflicting versions of the same dependency
    // Group dependencies by ident to find conflicts
    const dependenciesByIdent = new Map();

    for (const dependency of Yarn.dependencies()) {
      if (dependency.type === 'peerDependencies') continue;

      const ident = dependency.ident;
      if (!dependenciesByIdent.has(ident)) {
        dependenciesByIdent.set(ident, []);
      }
      dependenciesByIdent.get(ident).push(dependency);
    }

    // For each ident with multiple ranges, pick the highest and enforce it
    for (const [, dependencies] of dependenciesByIdent) {
      const uniqueRanges = [...new Set(dependencies.map((d) => d.range))];

      // Only process if there are conflicting ranges
      if (uniqueRanges.length > 1) {
        const chosenRange = pickHighestRange(uniqueRanges);

        for (const dependency of dependencies) {
          if (dependency.range !== chosenRange) {
            dependency.update(chosenRange);
          }
        }
      }
    }

    // Force all workspace dependencies to be made explicit with workspace:*
    for (const workspace of Yarn.workspaces()) {
      for (const dependency of Yarn.dependencies({ workspace })) {
        if (Yarn.workspace({ ident: dependency.ident })) {
          dependency.update('workspace:*');
        }
      }
    }

    // Ensure workspace versions stay in sync with the root release cadence
    // - Three workspaces must always lead the root major by a fixed offset (M+1, M+3, M+7)
    // - All other workspaces mirror the root version exactly
    if (rootVersion) {
      for (const workspace of Yarn.workspaces()) {
        const offset = SPECIAL_WORKSPACE_OFFSETS[workspace.ident];

        if (offset !== undefined) {
          // Special workspace: apply major version offset
          const expectedVersion = applyVersionOffset(rootVersion, offset);
          workspace.set('version', expectedVersion);
        } else if (workspace.manifest.version !== undefined) {
          // Regular workspace: mirror root version exactly
          workspace.set('version', rootVersion);
        }
      }
    }
  },
};
