/**
 * Put real state on a running local network so a state sync snapshot carries
 * something worth verifying.
 *
 * A network whose only content is empty blocks proves very little: the
 * reconstructed platform state would be almost entirely defaults, and a joined
 * node could look healthy while having restored nothing. Every step here
 * writes into a different subtree of Drive's state — identities, DPNS, data
 * contracts (including one with a ranked index, whose secondary count trees
 * only exist from protocol v14 on), documents, and identity balances — so the
 * post-sync assertions can distinguish "restored the snapshot" from "started
 * an empty chain".
 *
 * Seeding is deliberately granular. A step that cannot work on a local network
 * records itself as skipped and the rest continue: the point of the suite is
 * state sync, and losing the whole run because one optional write is
 * unsupported would be a bad trade.
 */

import crypto from 'crypto';
import wait from '../../../src/util/wait.js';

/**
 * How long to wait after a broadcast for the write to be queryable.
 *
 * @type {number}
 */
const ST_PROPAGATION_MS = 3000;

/**
 * Document type carrying a ranked index. `rankedCountable` needs
 * `rangeCountable: true` (and therefore a countable index), may not sit on a
 * unique index, and its terminal property's encoded key must stay under 247
 * bytes — a string costs 4x its `maxLength`, hence the 32 cap.
 *
 * @type {Object}
 */
const RANKED_DOCUMENT_SCHEMAS = {
  rankedItem: {
    type: 'object',
    indices: [
      {
        name: 'byCategory',
        properties: [{ category: 'asc' }],
        countable: 'countable',
        rangeCountable: true,
        rankedCountable: true,
      },
    ],
    properties: {
      category: {
        type: 'string', minLength: 1, maxLength: 32, position: 0,
      },
      label: { type: 'string', maxLength: 63, position: 1 },
    },
    required: ['category'],
    additionalProperties: false,
  },
};

/**
 * A plain document type with ordinary indices, as a control alongside the
 * ranked contract.
 *
 * @type {Object}
 */
const PLAIN_DOCUMENT_SCHEMAS = {
  note: {
    type: 'object',
    indices: [
      { name: 'byOwnerAndTitle', properties: [{ $ownerId: 'asc' }, { title: 'asc' }] },
    ],
    properties: {
      title: { type: 'string', maxLength: 63, position: 0 },
      body: { type: 'string', maxLength: 255, position: 1 },
    },
    required: ['title'],
    additionalProperties: false,
  },
};

/**
 * Record the outcome of one seeding step.
 *
 * @param {Object} manifest
 * @param {string} name
 * @param {function(): Promise<*>} step
 * @return {Promise<*|undefined>} the step's value, or undefined when it was skipped
 */
async function runStep(manifest, name, step) {
  try {
    const value = await step();

    manifest.steps.push({ name, status: 'ok' });

    return value;
  } catch (error) {
    manifest.steps.push({ name, status: 'skipped', reason: error.message });

    return undefined;
  }
}

/**
 * Seed identities, a DPNS name, two data contracts, documents and balance
 * movements onto the network `client` is connected to.
 *
 * @param {Client} client - a funded `Dash.Client`
 * @param {Object} [options]
 * @param {function(string): void} [options.log]
 * @return {Promise<Object>} manifest of what was seeded
 */
export default async function seedPlatformState(client, { log = () => {} } = {}) {
  const manifest = {
    steps: [],
    identities: [],
    name: undefined,
    contracts: {},
    documents: [],
  };

  /**
   * @param {number} amount
   * @return {Promise<Object>}
   */
  const registerIdentity = async (amount) => {
    const identity = await client.platform.identities.register(amount);

    await wait(ST_PROPAGATION_MS);

    return identity;
  };

  const primaryIdentity = await runStep(manifest, 'register primary identity', async () => {
    const identity = await registerIdentity(300000000);

    manifest.identities.push({
      id: identity.getId().toString(),
      balance: identity.getBalance().toString(),
      role: 'primary',
    });

    log(`seeded primary identity ${identity.getId().toString()}`);

    return identity;
  });

  const secondaryIdentity = await runStep(manifest, 'register secondary identity', async () => {
    const identity = await registerIdentity(200000000);

    manifest.identities.push({
      id: identity.getId().toString(),
      balance: identity.getBalance().toString(),
      role: 'secondary',
    });

    log(`seeded secondary identity ${identity.getId().toString()}`);

    return identity;
  });

  if (!primaryIdentity) {
    // Every remaining step signs with this identity, so there is nothing left
    // to attempt. The caller decides whether an unseeded run is fatal.
    return manifest;
  }

  await runStep(manifest, 'register DPNS name', async () => {
    // DPNS labels must be unique across the chain and may not start with a
    // digit, so prefix the random suffix.
    const label = `qa${crypto.randomBytes(6).toString('hex')}`;

    const domain = await client.platform.names.register(
      `${label}.dash`,
      { identity: primaryIdentity.getId() },
      primaryIdentity,
    );

    await wait(ST_PROPAGATION_MS);

    manifest.name = {
      label,
      normalizedLabel: domain.get('normalizedLabel'),
      fullName: `${label}.dash`,
      identityId: primaryIdentity.getId().toString(),
    };

    log(`seeded DPNS name ${label}.dash`);

    return domain;
  });

  /**
   * Publish a contract and register it on the client under `appName`.
   *
   * @param {string} appName
   * @param {Object} schemas
   * @return {Promise<Object>}
   */
  const publishContract = async (appName, schemas) => {
    const contract = await client.platform.contracts.create(schemas, primaryIdentity);

    await client.platform.contracts.publish(contract, primaryIdentity);

    await wait(ST_PROPAGATION_MS);

    client.getApps().set(appName, {
      contractId: contract.getId(),
      contract,
    });

    manifest.contracts[appName] = {
      id: contract.getId().toString(),
      documentTypes: Object.keys(schemas),
    };

    log(`seeded data contract ${appName} ${contract.getId().toString()}`);

    return contract;
  };

  /**
   * Create and broadcast one document.
   *
   * @param {string} appName
   * @param {string} documentType
   * @param {Object} data
   * @return {Promise<Object>}
   */
  const createDocument = async (appName, documentType, data) => {
    const document = await client.platform.documents.create(
      `${appName}.${documentType}`,
      primaryIdentity,
      data,
    );

    await client.platform.documents.broadcast({ create: [document] }, primaryIdentity);

    await wait(ST_PROPAGATION_MS);

    manifest.documents.push({
      id: document.getId().toString(),
      appName,
      documentType,
      contractId: manifest.contracts[appName].id,
      ownerId: primaryIdentity.getId().toString(),
      data,
    });

    log(`seeded document ${appName}.${documentType} ${document.getId().toString()}`);

    return document;
  };

  const plainContract = await runStep(
    manifest,
    'publish plain data contract',
    () => publishContract('qaPlain', PLAIN_DOCUMENT_SCHEMAS),
  );

  if (plainContract) {
    await runStep(manifest, 'create plain documents', async () => {
      await createDocument('qaPlain', 'note', { title: 'state sync', body: 'seeded before the snapshot' });
      await createDocument('qaPlain', 'note', { title: 'second note', body: 'also seeded' });
    });
  }

  const rankedContract = await runStep(
    manifest,
    'publish ranked-index data contract',
    () => publishContract('qaRanked', RANKED_DOCUMENT_SCHEMAS),
  );

  if (rankedContract) {
    await runStep(manifest, 'create ranked documents', async () => {
      await createDocument('qaRanked', 'rankedItem', { category: 'alpha', label: 'first' });
      await createDocument('qaRanked', 'rankedItem', { category: 'alpha', label: 'second' });
      await createDocument('qaRanked', 'rankedItem', { category: 'beta', label: 'third' });
    });
  }

  // js-dash-sdk exposes no token factories at all: wasm-dpp's token bindings
  // are getter-only and `contracts.create` forwards document schemas alone, so
  // a token contract cannot be declared, minted or transferred from this
  // stack. Recorded rather than silently dropped.
  manifest.steps.push({
    name: 'token contract mint + transfer',
    status: 'skipped',
    reason: 'js-dash-sdk has no token support (wasm-dpp token bindings are read-only '
      + 'and contracts.create forwards only document schemas)',
  });

  if (secondaryIdentity) {
    await runStep(manifest, 'top up secondary identity', async () => {
      const before = secondaryIdentity.getBalance();

      await client.platform.identities.topUp(secondaryIdentity.getId(), 1000000);

      await wait(ST_PROPAGATION_MS);

      const after = await client.platform.identities.get(secondaryIdentity.getId());

      manifest.topUp = {
        identityId: secondaryIdentity.getId().toString(),
        balanceBefore: before.toString(),
        balanceAfter: after.getBalance().toString(),
      };

      log(`topped up ${secondaryIdentity.getId().toString()}`);
    });
  }

  await runStep(manifest, 'withdraw credits', async () => {
    const account = await client.getWalletAccount();
    const withdrawTo = await account.getUnusedAddress();

    // Minimum is 190000 credits; go well above it so the withdrawal is not
    // rejected for dust while still leaving the identity funded.
    const metadata = await client.platform.identities.withdrawCredits(
      primaryIdentity,
      BigInt(1000000),
      { toAddress: withdrawTo.address },
    );

    await wait(ST_PROPAGATION_MS);

    manifest.withdrawal = {
      identityId: primaryIdentity.getId().toString(),
      toAddress: withdrawTo.address,
      height: metadata && metadata.height ? metadata.height.toString() : undefined,
    };

    log(`withdrew credits from ${primaryIdentity.getId().toString()}`);
  });

  return manifest;
}

/**
 * Human-readable summary of a seeding manifest, for the run log.
 *
 * @param {Object} manifest
 * @return {string}
 */
export function describeSeedManifest(manifest) {
  return manifest.steps
    .map(({ name, status, reason }) => (
      `  ${status === 'ok' ? 'ok     ' : 'skipped'} ${name}${reason ? ` — ${reason}` : ''}`
    ))
    .join('\n');
}
