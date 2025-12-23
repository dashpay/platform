const semver = require('semver');

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

module.exports = {
  constraints: async ({ Yarn }) => {
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
  },
};
