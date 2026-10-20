const assert = require('assert');
const { findLatestTag, interveningTags } = require('./find_latest_tag');

// Tag lists are ordered newest-created-first, matching `git tag --sort=-creatordate`.

let failures = 0;

function check(name, actual, expected) {
  try {
    assert.deepStrictEqual(actual, expected);
    console.log(`  ok - ${name}`);
  } catch (e) {
    failures += 1;
    console.error(`  FAIL - ${name}\n    expected: ${JSON.stringify(expected)}\n    actual:   ${JSON.stringify(actual)}`);
  }
}

// The regression: first rc of a new minor line. The base must be the last beta on the
// same line, NOT the previous stable minor. Basing off v4.0.0 regenerates the beta
// sections and duplicates them in CHANGELOG.md (the 4.1.0-rc.1 release bug).
check(
  'first rc bases off the newest beta, not the previous stable',
  findLatestTag('4.1.0-rc.1', ['v4.1.0-beta.2', 'v4.1.0-beta.1', 'v4.0.0']),
  'v4.1.0-beta.2',
);

// First beta of a line that had dev prereleases: base is the newest dev. semver would
// order "dev" after "beta" alphabetically, so this only works off release chronology.
check(
  'first beta bases off the newest dev on the line',
  findLatestTag('4.1.0-beta.1', ['v4.1.0-dev.2', 'v4.1.0-dev.1', 'v4.0.0']),
  'v4.1.0-dev.2',
);

// Truly first prerelease of a line (no prior prereleases): fall back to previous stable minor.
check(
  'first dev of a line bases off the previous stable minor',
  findLatestTag('4.1.0-dev.1', ['v4.0.1', 'v4.0.0']),
  'v4.0.1',
);

// Subsequent prerelease with the same id: base is the previous same-id prerelease.
check(
  'second rc bases off the first rc',
  findLatestTag('4.1.0-rc.2', ['v4.1.0-rc.1', 'v4.1.0-beta.2', 'v4.0.0']),
  'v4.1.0-rc.1',
);

// Stable cut from a prerelease line: base is the newest prerelease on that line.
check(
  'stable bases off the newest prerelease on its line',
  findLatestTag('4.1.0', ['v4.1.0-rc.1', 'v4.1.0-beta.2', 'v4.0.0']),
  'v4.1.0-rc.1',
);

// Patch stable: base is the previous stable of the same minor.
check(
  'patch release bases off the previous same-minor stable',
  findLatestTag('4.0.1', ['v4.0.0', 'v3.0.2']),
  'v4.0.0',
);

// The target tag already existing (e.g. a re-run) must not pick itself.
check(
  'does not pick the target tag itself',
  findLatestTag('4.1.0-rc.1', ['v4.1.0-rc.1', 'v4.1.0-beta.2', 'v4.0.0']),
  'v4.1.0-beta.2',
);

// Intervening-tag detection: a too-far-back base surfaces the tags that would duplicate.
check(
  'intervening tags flags the sections that would be regenerated',
  interveningTags('4.1.0-rc.1', 'v4.0.0', ['v4.1.0-beta.2', 'v4.1.0-beta.1', 'v4.0.0']),
  ['v4.1.0-beta.2', 'v4.1.0-beta.1'],
);

// A correct base has no intervening tags.
check(
  'correct base has no intervening tags',
  interveningTags('4.1.0-rc.1', 'v4.1.0-beta.2', ['v4.1.0-beta.2', 'v4.1.0-beta.1', 'v4.0.0']),
  [],
);

if (failures > 0) {
  console.error(`\n${failures} test(s) failed`);
  process.exit(1);
}
console.log('\nAll find_latest_tag tests passed');
