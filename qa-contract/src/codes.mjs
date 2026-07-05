// Canonical integer codes for the app / tier / category lookup document types.
// These are the foreign-key values stored on testCase (app, tier, category) and
// testRun (app). Codes are STABLE — append new entries with the next id; never
// renumber an existing one (it would orphan already-stored references).

export const APPS = [
  { code: 0, name: 'SwiftExampleApp', platform: 'iOS', description: 'Dash Platform iOS example wallet (Core SPV + Platform).' },
  { code: 1, name: 'KotlinExampleApp', platform: 'Android', description: 'Dash Platform Android example wallet (Core SPV + Platform).' },
];

export const TIERS = [
  { code: 0, name: 'Essential' },
  { code: 1, name: 'Common' },
  { code: 2, name: 'Thorough' },
  { code: 3, name: 'Uncommon' },
  { code: 4, name: 'Manual' },
  { code: 5, name: 'Unspecified' },
];

export const CATEGORIES = [
  { code: 0, name: 'Core' },
  { code: 1, name: 'Identity' },
  { code: 2, name: 'Address' },
  { code: 3, name: 'DPNS' },
  { code: 4, name: 'Voting' },
  { code: 5, name: 'Contract' },
  { code: 6, name: 'Document' },
  { code: 7, name: 'Token' },
  { code: 8, name: 'Shielded' },
  { code: 9, name: 'DashPay' },
  { code: 10, name: 'System' },
  { code: 12, name: 'MultiWallet' },
];

// The app the iOS TEST_PLAN.md belongs to.
export const DEFAULT_APP = 'SwiftExampleApp';

const byNameMap = (rows) => Object.fromEntries(rows.map((r) => [r.name.toLowerCase(), r.code]));
const byCodeMap = (rows) => Object.fromEntries(rows.map((r) => [r.code, r.name]));

const APP_BY_NAME = byNameMap(APPS);
const TIER_BY_NAME = byNameMap(TIERS);
const CATEGORY_BY_NAME = byNameMap(CATEGORIES);

export const APP_BY_CODE = byCodeMap(APPS);
export const TIER_BY_CODE = byCodeMap(TIERS);
export const CATEGORY_BY_CODE = byCodeMap(CATEGORIES);

function lookup(map, kind, name) {
  const code = map[String(name).toLowerCase()];
  if (code === undefined) {
    throw new Error(`Unknown ${kind} '${name}'. Add it to src/codes.mjs (and re-seed the lookup docs).`);
  }
  return code;
}

export const appCode = (name) => lookup(APP_BY_NAME, 'app', name);
export const tierCode = (name) => lookup(TIER_BY_NAME, 'tier', name);
export const categoryCode = (name) => lookup(CATEGORY_BY_NAME, 'category', name);
