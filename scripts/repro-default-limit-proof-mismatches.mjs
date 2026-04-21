#!/usr/bin/env node

import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

/*
 * Reproduce proof-verification mismatches caused by omitted limits.
 *
 * The bug shape is:
 * - Platform applies a default limit when count/limit is omitted
 * - The proof verifier rebuilds the request as if no limit was applied
 * - The same query succeeds when an explicit limit is supplied
 *
 * Usage:
 *   node scripts/repro-default-limit-proof-mismatches.mjs
 *
 * If you are running from this monorepo with Yarn Plug'n'Play, use the
 * package-aware runtime instead of plain Node. For example:
 *   yarn node scripts/repro-default-limit-proof-mismatches.mjs
 *
 * Optional environment variables:
 *   EVO_SDK_IMPORT
 *     Alternate module specifier for EvoSDK. If omitted, the script tries:
 *     1. "@dashevo/evo-sdk"
 *     2. the local workspace build at packages/js-evo-sdk/dist/sdk.js
 *
 *   NETWORK
 *     mainnet | testnet | local. Defaults to mainnet.
 *
 *   TRUSTED
 *     true | false. Defaults to true.
 *
 *   DEFAULT_LIMIT
 *     Explicit limit to use for the control query. Defaults to 100.
 *
 *   DPNS_CONTRACT_ID
 *     Defaults to the DPNS mainnet contract used in the earlier repro.
 *
 *   DPNS_DOCUMENT_TYPE
 *     Defaults to "domain".
 *
 *   DPNS_INDEX
 *     Defaults to "parentNameAndLabel".
 *
 *   CONTESTED_PARENT
 *     Defaults to "dash".
 *
 *   VOTE_STATE_LABEL
 *     Optional explicit label for contestedResourceVoteState repro.
 *
 *   VOTERS_LABEL
 *   VOTERS_CONTESTANT_ID
 *     Optional explicit fixture for contestedResourceVotersForIdentity repro.
 *
 *   IDENTITY_VOTES_IDENTITY_ID
 *     Optional explicit fixture for contestedResourceIdentityVotes repro.
 *
 *   VOTE_POLLS_START_MS
 *   VOTE_POLLS_END_MS
 *     Optional explicit time range for votePollsByEndDate repro.
 *
 *   GROUP_INFOS_CONTRACT_ID
 *     Optional fixture for group.infos repro.
 *
 *   GROUP_ACTIONS_CONTRACT_ID
 *   GROUP_ACTIONS_POSITION
 *   GROUP_ACTIONS_STATUS
 *     Optional fixture for group.actions repro.
 *
 *   TOKEN_DISTRIBUTIONS_TOKEN_ID
 *     Reserved for a future EvoSDK token pre-programmed distributions repro.
 *     The current JS EvoSDK does not expose this query yet, so the script
 *     reports it as skipped.
 */

const DEFAULT_DPNS_CONTRACT_ID =
  process.env.DPNS_CONTRACT_ID ?? 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
const DEFAULT_DPNS_DOCUMENT_TYPE = process.env.DPNS_DOCUMENT_TYPE ?? 'domain';
const DEFAULT_DPNS_INDEX = process.env.DPNS_INDEX ?? 'parentNameAndLabel';
const DEFAULT_PARENT = process.env.CONTESTED_PARENT ?? 'dash';
const NETWORK = process.env.NETWORK ?? 'mainnet';
const TRUSTED = (process.env.TRUSTED ?? 'true').toLowerCase() !== 'false';
const localSdkImport = new URL('../packages/js-evo-sdk/dist/sdk.js', import.meta.url).href;
const EVO_SDK_IMPORT = process.env.EVO_SDK_IMPORT ?? null;

const nowMs = Date.now();

function printHeading(title) {
  console.log(`\n=== ${title} ===`);
}

function parseNumericEnv(name, defaultValue, { integer = false } = {}) {
  const raw = process.env[name];

  if (raw == null || raw === '') {
    return defaultValue;
  }

  const parsed = Number(raw);
  if (!Number.isFinite(parsed) || (integer && !Number.isInteger(parsed))) {
    throw new Error(
      `Invalid ${name}: expected ${integer ? 'an integer' : 'a finite number'}, got ${JSON.stringify(raw)}`,
    );
  }

  return parsed;
}

const DEFAULT_LIMIT = parseNumericEnv('DEFAULT_LIMIT', 100, { integer: true });
const defaultVotePollsStartMs = parseNumericEnv('VOTE_POLLS_START_MS', 0, { integer: true });
const defaultVotePollsEndMs = parseNumericEnv(
  'VOTE_POLLS_END_MS',
  nowMs + 365 * 24 * 60 * 60 * 1000,
  { integer: true },
);

function shorten(value, max = 140) {
  const text = typeof value === 'string' ? value : JSON.stringify(value);
  if (text.length <= max) {
    return text;
  }
  return `${text.slice(0, max - 3)}...`;
}

function normalizeError(error) {
  if (!error) {
    return 'Unknown error';
  }
  return error?.message ?? String(error);
}

function countResult(result) {
  if (Array.isArray(result)) {
    return result.length;
  }
  if (result instanceof Map) {
    return result.size;
  }
  if (typeof result?.size === 'number') {
    return result.size;
  }
  if (typeof result?.length === 'number') {
    return result.length;
  }
  return null;
}

function summarizeResult(result) {
  if (Array.isArray(result)) {
    return shorten(result.slice(0, 5).map(stringifySdkValue));
  }
  if (result instanceof Map) {
    return shorten(
      Array.from(result.entries())
        .slice(0, 5)
        .map(([key, value]) => [key, stringifySdkValue(value)]),
    );
  }
  return shorten(stringifySdkValue(result));
}

function stringifySdkValue(value) {
  if (value == null) {
    return value;
  }
  if (typeof value === 'bigint') {
    return value.toString();
  }
  if (Array.isArray(value)) {
    return value.map(stringifySdkValue);
  }
  if (value instanceof Uint8Array) {
    return Array.from(value);
  }
  if (typeof value?.toJSON === 'function') {
    try {
      return value.toJSON();
    } catch {
      // ignore
    }
  }
  if (typeof value?.toString === 'function' && value.toString !== Object.prototype.toString) {
    try {
      const text = value.toString();
      if (text && text !== '[object Object]') {
        return text;
      }
    } catch {
      // ignore
    }
  }
  return value;
}

function identifierToString(id) {
  if (!id) {
    return null;
  }
  if (typeof id === 'string') {
    return id;
  }
  if (typeof id.toString === 'function') {
    return id.toString();
  }
  return String(id);
}

async function buildSdk(EvoSDK) {
  const builderName = `${NETWORK}${TRUSTED ? 'Trusted' : ''}`;
  if (typeof EvoSDK[builderName] !== 'function') {
    throw new Error(`Unsupported NETWORK/TRUSTED combination: ${builderName}`);
  }

  const sdk = await EvoSDK[builderName]();
  await sdk.connect();
  return sdk;
}

function resolveBareImportFromNodeModules(specifier) {
  let currentDir = process.cwd();

  while (true) {
    const packageDir = path.join(currentDir, 'node_modules', specifier);
    const packageJsonPath = path.join(packageDir, 'package.json');

    if (existsSync(packageJsonPath)) {
      const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf8'));
      const exportEntry = packageJson.exports?.['.'];
      const relativeEntry =
        (typeof exportEntry === 'string' && exportEntry)
        || exportEntry?.import
        || packageJson.module
        || packageJson.main;

      if (!relativeEntry) {
        throw new Error(`Could not determine entry point from ${packageJsonPath}`);
      }

      return pathToFileURL(path.join(packageDir, relativeEntry)).href;
    }

    const parentDir = path.dirname(currentDir);
    if (parentDir === currentDir) {
      break;
    }
    currentDir = parentDir;
  }

  return null;
}

async function loadEvoSdk() {
  const candidates = EVO_SDK_IMPORT
    ? [EVO_SDK_IMPORT]
    : ['@dashevo/evo-sdk', localSdkImport];

  const failures = [];
  for (const specifier of candidates) {
    try {
      let target = specifier;
      if (
        !specifier.startsWith('.')
        && !specifier.startsWith('/')
        && !specifier.startsWith('file:')
      ) {
        target = resolveBareImportFromNodeModules(specifier) ?? specifier;
      }

      const mod = await import(target);
      if (mod?.EvoSDK) {
        return mod.EvoSDK;
      }
      failures.push(`${specifier}: imported, but no EvoSDK export was found`);
    } catch (error) {
      failures.push(`${specifier}: ${normalizeError(error)}`);
    }
  }

  throw new Error(
    [
      'Unable to import EvoSDK.',
      ...failures.map((failure) => `- ${failure}`),
      "If you are in this monorepo, run via `yarn node` so Plug'n'Play dependencies resolve.",
      'If you want the published package, install `@dashevo/evo-sdk` and optionally set EVO_SDK_IMPORT=@dashevo/evo-sdk.',
    ].join('\n'),
  );
}

async function runPair(name, buildNoLimit, buildWithLimit) {
  const record = {
    name,
    noLimit: null,
    explicitLimit: null,
    conclusion: null,
  };

  const cases = [
    ['noLimit', buildNoLimit],
    ['explicitLimit', buildWithLimit],
  ];

  for (const [key, build] of cases) {
    let result;

    try {
      result = await build();
    } catch (queryError) {
      record[key] = {
        ok: false,
        error: normalizeError(queryError),
      };
      continue;
    }

    const value = { ok: true };

    try {
      value.count = countResult(result);
    } catch (formatError) {
      value.count = null;
      value.formattingError = normalizeError(formatError);
    }

    try {
      value.summary = summarizeResult(result);
    } catch (formatError) {
      value.summary = '<unavailable>';
      const normalized = normalizeError(formatError);
      value.formattingError = value.formattingError
        ? `${value.formattingError}; ${normalized}`
        : normalized;
    }

    record[key] = value;
  }

  if (record.noLimit?.ok === false && record.explicitLimit?.ok === true) {
    record.conclusion = 'CONFIRMED_DEFAULT_LIMIT_MISMATCH';
  } else if (record.noLimit?.ok === true && record.explicitLimit?.ok === true) {
    record.conclusion = 'NO_FAILURE_WITH_CURRENT_FIXTURE';
  } else if (record.noLimit?.ok === false && record.explicitLimit?.ok === false) {
    record.conclusion = 'FIXTURE_OR_ENDPOINT_FAILED_BOTH_WAYS';
  } else {
    record.conclusion = 'INCONCLUSIVE';
  }

  return record;
}

function printRecord(record) {
  console.log(`\n[${record.conclusion}] ${record.name}`);

  for (const key of ['noLimit', 'explicitLimit']) {
    const value = record[key];
    if (!value) {
      console.log(`  ${key}: skipped`);
      continue;
    }

    if (value.ok) {
      const count = value.count == null ? 'n/a' : value.count;
      console.log(`  ${key}: ok, count=${count}, sample=${value.summary}`);
    } else {
      console.log(`  ${key}: error=${value.error}`);
    }
  }
}

function getTokenPreProgrammedDistributionsMethod(sdk) {
  const candidateNames = [
    'preProgrammedDistributions',
    'tokenPreProgrammedDistributions',
    'getPreProgrammedDistributions',
  ];

  for (const name of candidateNames) {
    if (typeof sdk?.tokens?.[name] === 'function') {
      return sdk.tokens[name].bind(sdk.tokens);
    }
  }

  return null;
}

async function autoDiscoverContestedLabel(sdk) {
  const explicit = process.env.VOTE_STATE_LABEL;
  if (explicit) {
    return { label: explicit, labels: undefined };
  }

  const labels = await sdk.group.contestedResources({
    dataContractId: DEFAULT_DPNS_CONTRACT_ID,
    documentTypeName: DEFAULT_DPNS_DOCUMENT_TYPE,
    indexName: DEFAULT_DPNS_INDEX,
    startIndexValues: [DEFAULT_PARENT],
    limit: DEFAULT_LIMIT,
    orderAscending: true,
  });

  const activeEntries = await sdk.voting.votePollsByEndDate({
    startTimeMs: defaultVotePollsStartMs,
    startTimeIncluded: true,
    endTimeMs: defaultVotePollsEndMs,
    endTimeIncluded: true,
    orderAscending: true,
    limit: DEFAULT_LIMIT,
  });

  for (const entry of activeEntries) {
    for (const poll of entry.votePolls) {
      const json = poll.toJSON?.();
      const values = json?.contestedDocumentResourceVotePoll?.indexValues;
      if (Array.isArray(values) && values[0] === DEFAULT_PARENT && typeof values[1] === 'string') {
        return { label: values[1], labels };
      }
    }
  }

  return { label: labels[0] ?? null, labels };
}

async function autoDiscoverContestantId(sdk, label) {
  if (process.env.VOTERS_CONTESTANT_ID) {
    return process.env.VOTERS_CONTESTANT_ID;
  }

  if (!label) {
    return null;
  }

  const state = await sdk.voting.contestedResourceVoteState({
    dataContractId: DEFAULT_DPNS_CONTRACT_ID,
    documentTypeName: DEFAULT_DPNS_DOCUMENT_TYPE,
    indexName: DEFAULT_DPNS_INDEX,
    indexValues: [DEFAULT_PARENT, label],
    resultType: 'documentsAndVoteTally',
    includeLockedAndAbstaining: true,
    limit: DEFAULT_LIMIT,
  });

  const contender = state?.contenders?.[0];
  return identifierToString(contender?.identityId);
}

async function autoDiscoverIdentityVotesIdentityId(sdk, label, contestantId) {
  if (process.env.IDENTITY_VOTES_IDENTITY_ID) {
    return process.env.IDENTITY_VOTES_IDENTITY_ID;
  }

  if (!label || !contestantId) {
    return null;
  }

  try {
    const voters = await sdk.group.contestedResourceVotersForIdentity({
      dataContractId: DEFAULT_DPNS_CONTRACT_ID,
      documentTypeName: DEFAULT_DPNS_DOCUMENT_TYPE,
      indexName: DEFAULT_DPNS_INDEX,
      indexValues: [DEFAULT_PARENT, label],
      contestantId,
      orderAscending: true,
      limit: DEFAULT_LIMIT,
    });

    return identifierToString(voters?.[0]);
  } catch {
    return null;
  }
}

async function main() {
  const EvoSDK = await loadEvoSdk();
  const sdk = await buildSdk(EvoSDK);

  try {
    printHeading(`Connected to ${NETWORK} (${TRUSTED ? 'trusted' : 'untrusted'})`);
    console.log(`using explicit control limit = ${DEFAULT_LIMIT}`);

    const records = [];

    records.push(
      await runPair(
        'group.contestedResources(startIndexValues=["dash"])',
        () =>
          sdk.group.contestedResources({
            dataContractId: DEFAULT_DPNS_CONTRACT_ID,
            documentTypeName: DEFAULT_DPNS_DOCUMENT_TYPE,
            indexName: DEFAULT_DPNS_INDEX,
            startIndexValues: [DEFAULT_PARENT],
            orderAscending: true,
          }),
        () =>
          sdk.group.contestedResources({
            dataContractId: DEFAULT_DPNS_CONTRACT_ID,
            documentTypeName: DEFAULT_DPNS_DOCUMENT_TYPE,
            indexName: DEFAULT_DPNS_INDEX,
            startIndexValues: [DEFAULT_PARENT],
            orderAscending: true,
            limit: DEFAULT_LIMIT,
          }),
      ),
    );

    const currentEpoch = await sdk.epoch.current();
    const epochIndex = Number(currentEpoch.index ?? currentEpoch);
    records.push(
      await runPair(
        `epoch.evonodesProposedBlocksByRange(epoch=${epochIndex})`,
        () =>
          sdk.epoch.evonodesProposedBlocksByRange({
            epoch: epochIndex,
            orderAscending: true,
          }),
        () =>
          sdk.epoch.evonodesProposedBlocksByRange({
            epoch: epochIndex,
            orderAscending: true,
            limit: DEFAULT_LIMIT,
          }),
      ),
    );

    const { label } = await autoDiscoverContestedLabel(sdk);
    if (label) {
      records.push(
        await runPair(
          `voting.contestedResourceVoteState(label=${label})`,
          () =>
            sdk.voting.contestedResourceVoteState({
              dataContractId: DEFAULT_DPNS_CONTRACT_ID,
              documentTypeName: DEFAULT_DPNS_DOCUMENT_TYPE,
              indexName: DEFAULT_DPNS_INDEX,
              indexValues: [DEFAULT_PARENT, label],
              resultType: 'documentsAndVoteTally',
              includeLockedAndAbstaining: true,
            }),
          () =>
            sdk.voting.contestedResourceVoteState({
              dataContractId: DEFAULT_DPNS_CONTRACT_ID,
              documentTypeName: DEFAULT_DPNS_DOCUMENT_TYPE,
              indexName: DEFAULT_DPNS_INDEX,
              indexValues: [DEFAULT_PARENT, label],
              resultType: 'documentsAndVoteTally',
              includeLockedAndAbstaining: true,
              limit: DEFAULT_LIMIT,
            }),
        ),
      );

      const contestantId = await autoDiscoverContestantId(sdk, process.env.VOTERS_LABEL ?? label);
      if (contestantId) {
        const votersLabel = process.env.VOTERS_LABEL ?? label;
        records.push(
          await runPair(
            `group.contestedResourceVotersForIdentity(label=${votersLabel}, contestantId=${contestantId})`,
            () =>
              sdk.group.contestedResourceVotersForIdentity({
                dataContractId: DEFAULT_DPNS_CONTRACT_ID,
                documentTypeName: DEFAULT_DPNS_DOCUMENT_TYPE,
                indexName: DEFAULT_DPNS_INDEX,
                indexValues: [DEFAULT_PARENT, votersLabel],
                contestantId,
                orderAscending: true,
              }),
            () =>
              sdk.group.contestedResourceVotersForIdentity({
                dataContractId: DEFAULT_DPNS_CONTRACT_ID,
                documentTypeName: DEFAULT_DPNS_DOCUMENT_TYPE,
                indexName: DEFAULT_DPNS_INDEX,
                indexValues: [DEFAULT_PARENT, votersLabel],
                contestantId,
                orderAscending: true,
                limit: DEFAULT_LIMIT,
              }),
          ),
        );

        const identityVotesIdentityId = await autoDiscoverIdentityVotesIdentityId(
          sdk,
          votersLabel,
          contestantId,
        );

        if (identityVotesIdentityId) {
          records.push(
            await runPair(
              `voting.contestedResourceIdentityVotes(identityId=${identityVotesIdentityId})`,
              () =>
                sdk.voting.contestedResourceIdentityVotes({
                  identityId: identityVotesIdentityId,
                  orderAscending: true,
                }),
              () =>
                sdk.voting.contestedResourceIdentityVotes({
                  identityId: identityVotesIdentityId,
                  orderAscending: true,
                  limit: DEFAULT_LIMIT,
                }),
            ),
          );
        } else {
          records.push({
            name: 'voting.contestedResourceIdentityVotes',
            noLimit: null,
            explicitLimit: null,
            conclusion: 'SKIPPED_NO_IDENTITY_FIXTURE',
          });
        }
      } else {
        records.push({
          name: 'group.contestedResourceVotersForIdentity',
          noLimit: null,
          explicitLimit: null,
          conclusion: 'SKIPPED_NO_CONTESTANT_FIXTURE',
        });
        records.push({
          name: 'voting.contestedResourceIdentityVotes',
          noLimit: null,
          explicitLimit: null,
          conclusion: 'SKIPPED_NO_IDENTITY_FIXTURE',
        });
      }
    } else {
      records.push({
        name: 'voting.contestedResourceVoteState',
        noLimit: null,
        explicitLimit: null,
        conclusion: 'SKIPPED_NO_LABEL_DISCOVERED',
      });
      records.push({
        name: 'group.contestedResourceVotersForIdentity',
        noLimit: null,
        explicitLimit: null,
        conclusion: 'SKIPPED_NO_CONTESTANT_FIXTURE',
      });
      records.push({
        name: 'voting.contestedResourceIdentityVotes',
        noLimit: null,
        explicitLimit: null,
        conclusion: 'SKIPPED_NO_IDENTITY_FIXTURE',
      });
    }

    records.push(
      await runPair(
        `voting.votePollsByEndDate(start=${defaultVotePollsStartMs}, end=${defaultVotePollsEndMs})`,
        () =>
          sdk.voting.votePollsByEndDate({
            startTimeMs: defaultVotePollsStartMs,
            startTimeIncluded: true,
            endTimeMs: defaultVotePollsEndMs,
            endTimeIncluded: true,
            orderAscending: true,
          }),
        () =>
          sdk.voting.votePollsByEndDate({
            startTimeMs: defaultVotePollsStartMs,
            startTimeIncluded: true,
            endTimeMs: defaultVotePollsEndMs,
            endTimeIncluded: true,
            orderAscending: true,
            limit: DEFAULT_LIMIT,
          }),
      ),
    );

    if (process.env.GROUP_INFOS_CONTRACT_ID) {
      records.push(
        await runPair(
          `group.infos(contractId=${process.env.GROUP_INFOS_CONTRACT_ID})`,
          () =>
            sdk.group.infos({
              dataContractId: process.env.GROUP_INFOS_CONTRACT_ID,
            }),
          () =>
            sdk.group.infos({
              dataContractId: process.env.GROUP_INFOS_CONTRACT_ID,
              limit: DEFAULT_LIMIT,
            }),
        ),
      );
    } else {
      records.push({
        name: 'group.infos',
        noLimit: null,
        explicitLimit: null,
        conclusion: 'SKIPPED_NO_GROUP_INFOS_CONTRACT_ID',
      });
    }

    if (
      process.env.GROUP_ACTIONS_CONTRACT_ID
      && process.env.GROUP_ACTIONS_POSITION
      && process.env.GROUP_ACTIONS_STATUS
    ) {
      const groupContractPosition = parseNumericEnv('GROUP_ACTIONS_POSITION', null, {
        integer: true,
      });
      records.push(
        await runPair(
          `group.actions(contractId=${process.env.GROUP_ACTIONS_CONTRACT_ID}, position=${groupContractPosition})`,
          () =>
            sdk.group.actions({
              dataContractId: process.env.GROUP_ACTIONS_CONTRACT_ID,
              groupContractPosition,
              status: process.env.GROUP_ACTIONS_STATUS,
            }),
          () =>
            sdk.group.actions({
              dataContractId: process.env.GROUP_ACTIONS_CONTRACT_ID,
              groupContractPosition,
              status: process.env.GROUP_ACTIONS_STATUS,
              limit: DEFAULT_LIMIT,
            }),
        ),
      );
    } else {
      records.push({
        name: 'group.actions',
        noLimit: null,
        explicitLimit: null,
        conclusion: 'SKIPPED_NO_GROUP_ACTIONS_FIXTURE',
      });
    }

    const tokenDistributionsMethod = getTokenPreProgrammedDistributionsMethod(sdk);
    if (!tokenDistributionsMethod) {
      records.push({
        name: 'tokens.preProgrammedDistributions',
        noLimit: null,
        explicitLimit: null,
        conclusion: 'SKIPPED_NOT_EXPOSED_BY_EVO_SDK',
      });
    } else if (!process.env.TOKEN_DISTRIBUTIONS_TOKEN_ID) {
      records.push({
        name: 'tokens.preProgrammedDistributions',
        noLimit: null,
        explicitLimit: null,
        conclusion: 'SKIPPED_NO_TOKEN_DISTRIBUTIONS_TOKEN_ID',
      });
    } else {
      records.push(
        await runPair(
          `tokens.preProgrammedDistributions(tokenId=${process.env.TOKEN_DISTRIBUTIONS_TOKEN_ID})`,
          () =>
            tokenDistributionsMethod({
              tokenId: process.env.TOKEN_DISTRIBUTIONS_TOKEN_ID,
            }),
          () =>
            tokenDistributionsMethod({
              tokenId: process.env.TOKEN_DISTRIBUTIONS_TOKEN_ID,
              limit: DEFAULT_LIMIT,
            }),
        ),
      );
    }

    printHeading('Results');
    for (const record of records) {
      printRecord(record);
    }

    const confirmed = records.filter(
      (record) => record.conclusion === 'CONFIRMED_DEFAULT_LIMIT_MISMATCH',
    );
    printHeading('Summary');
    console.log(`confirmed mismatches: ${confirmed.length}`);
    for (const record of confirmed) {
      console.log(`- ${record.name}`);
    }
  } finally {
    try {
      await sdk.disconnect?.();
    } catch {
      // ignore disconnect failures
    }
  }
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
