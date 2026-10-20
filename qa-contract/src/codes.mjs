// Canonical integer codes for the app / tier / category lookup document types,
// plus the canonical `tags` vocabulary and the v4 -> v5 testId remap.
//
// These are the foreign-key values stored on testCase (app, tier, category) and
// testRun (app). v5 starts a FRESH contract, so the category codes were
// renumbered after dissolving MultiWallet and Group into the `tags` vocabulary
// (a test's *domain* is its category; its *modality* is a tag). Within v5 these
// codes are STABLE — append new entries with the next id; never renumber an
// existing one (it would orphan already-stored references).

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

// Feature-area domains. MultiWallet and Group are NOT categories in v5 — they are
// tags (see TAGS): a multi-wallet document test lives under `Document` + the
// `multiwallet` tag; a group-authorized token action under `Token` + `group`.
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
];

// Canonical cross-cutting tag vocabulary stored on testCase.tags (a string array,
// orthogonal to category). Keep lowercase + hyphenated. Add an entry here before
// using it in TEST_PLAN.md — checkTags() rejects unknown tags at seed time.
export const TAGS = [
  'multiwallet',   // exercises >=2 on-device wallets / identities
  'group',         // multi-party group-authorized action (propose / co-sign)
  'contested',     // contested resource (premium DPNS name, masternode vote, race)
  'withdrawal',    // moves value off-platform to an L1 Core address
  'distribution',  // token distribution mechanism (pre-programmed / perpetual / claim)
  'aggregation',   // count / sum / average / group_by query
  'read-only',     // pure query, no state-transition broadcast
  'regression',    // pins a specific previously-fixed bug
  'proof',         // specifically validates cryptographic proof verification
  'freeze',        // freeze / unfreeze / destroy-frozen balances
  'funding',       // requires asset-lock / faucet funding to run
  'masternode',    // requires a masternode key
];

// v4 -> v5 testId remap. MultiWallet (MW-*) and Group (GRP-*) ids were folded into
// their domain category's sequence. GRP-03 (token group propose / co-sign) is
// MERGED into the existing TOK-15 / TOK-16 pair, so its historical runs attach to
// TOK-15. Ids absent here are unchanged (identity mapping). Used by
// migrate-runs.mjs to re-point historical testRun.testId; testCase ids are
// renumbered directly in TEST_PLAN.md.
export const TESTID_REMAP = {
  'MW-01': 'ID-14',
  'MW-02': 'TOK-17',
  'MW-03': 'DP-11', // DashPay grew to DP-10 (label / QR / contactInfo / backfill); the loop lands at DP-11
  'MW-04': 'DOC-15',
  'MW-05': 'DPNS-08',
  'MW-06': 'SH-14',
  'MW-07': 'SH-15',
  'MW-08': 'SYS-07',
  'MW-09': 'SYS-08',
  'MW-10': 'ID-15',
  'MW-11': 'SH-16',
  'GRP-01': 'TOK-18',
  'GRP-02': 'TOK-19',
  'GRP-03': 'TOK-15', // merged into the TOK-15 / TOK-16 group-action pair
  'GRP-04': 'TOK-20',
};

export const remapTestId = (id) => TESTID_REMAP[id] || id;

// The app the iOS TEST_PLAN.md belongs to.
export const DEFAULT_APP = 'SwiftExampleApp';

const byNameMap = (rows) => Object.fromEntries(rows.map((r) => [r.name.toLowerCase(), r.code]));
const byCodeMap = (rows) => Object.fromEntries(rows.map((r) => [r.code, r.name]));

const APP_BY_NAME = byNameMap(APPS);
const TIER_BY_NAME = byNameMap(TIERS);
const CATEGORY_BY_NAME = byNameMap(CATEGORIES);
const TAG_SET = new Set(TAGS);

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

// Validate + de-dupe a list of tags against the canonical vocabulary. Throws on
// an unknown tag so a typo in TEST_PLAN.md fails the seed loudly rather than
// silently storing an unfilterable tag.
export function checkTags(tags, testId = '') {
  const out = [];
  for (const raw of tags || []) {
    const t = String(raw).trim().toLowerCase();
    if (!t) continue;
    if (!TAG_SET.has(t)) {
      throw new Error(`Unknown tag '${t}'${testId ? ` on ${testId}` : ''}. Add it to TAGS in src/codes.mjs.`);
    }
    if (!out.includes(t)) out.push(t);
  }
  return out;
}
