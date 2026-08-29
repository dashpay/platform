import Dash from 'dash';
import DashCoreLib from '@dashevo/dashcore-lib';
import CoreService from '../../../src/core/CoreService.js';
import wait from '../../../src/util/wait.js';

const { PrivateKey } = DashCoreLib;

/**
 * SDK plumbing for e2e specs that need to talk Platform to a node of a local
 * dashmate network.
 *
 * The local gateway serves a self-signed certificate and the state sync suite
 * moves every host port off its default, so nothing here may assume the stock
 * ports the platform-test-suite reads out of `.env`. Every address is derived
 * from the dashmate `Config` of the node being addressed, which is what lets a
 * caller point one client at the validators and another at the joined node.
 */

/**
 * `WasmSdkError.name` for a transition family whose proof cannot bind the
 * execution of one specific transition.
 *
 * @type {string}
 */
const EXECUTION_NOT_PROVED = 'ExecutionNotProved';

/**
 * Shared EvoSDK instances, keyed by address. The WASM SDK multiplexes
 * concurrent requests, and instantiating the WASM module repeatedly inside one
 * mocha process is both slow and needless.
 *
 * @type {Map<string, Promise<{ evo: Object, sdk: Object }>>}
 */
const evoSdkCache = new Map();

/**
 * Host-facing DAPI address of a node, in `@dashevo/dapi-client` seed notation.
 *
 * @param {Config} config
 * @return {string}
 */
export function getDapiAddress(config) {
  const port = config.get('platform.gateway.listeners.dapiAndDrive.port');

  return `127.0.0.1:${port}:self-signed`;
}

/**
 * Base URL of the local network's quorum list sidecar, which the WASM SDK's
 * trusted context uses to learn quorum public keys.
 *
 * The sidecar runs on the seed node and its port is not offset per node, so
 * any config of the group carries the right value.
 *
 * @param {Config} config
 * @return {string}
 */
export function getQuorumListUrl(config) {
  return `http://127.0.0.1:${config.get('platform.quorumList.api.port')}`;
}

/**
 * Get (or lazily create) an EvoSDK connected to one specific node.
 *
 * Proofs are switched on explicitly: every read this SDK performs is checked
 * against a GroveDB proof and the Tenderdash quorum signature over the root
 * hash, so a node that serves plausible-looking but unproven state fails
 * rather than passes. Non-trusted mode is unavailable in WASM, so quorum
 * public keys come from the local network's quorum list sidecar.
 *
 * @param {Config} config - node to query
 * @param {Config} quorumListConfig - any config of the group (for the sidecar port)
 * @return {Promise<{ evo: Object, sdk: Object }>}
 */
export function getEvoSdk(config, quorumListConfig) {
  const address = `https://127.0.0.1:${config.get('platform.gateway.listeners.dapiAndDrive.port')}`;

  if (!evoSdkCache.has(address)) {
    evoSdkCache.set(address, (async () => {
      // The local gateway's certificate is self-signed and the WASM SDK's
      // transport goes through fetch, which has no per-request TLS escape
      // hatch. Quorum-verified proofs, not TLS, are the trust boundary here.
      process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0';

      const evo = await import('@dashevo/evo-sdk');

      await evo.ensureInitialized();

      // Connecting prefetches quorum keys and masternode addresses from the
      // quorum list sidecar, whose per-masternode version checks need time
      // after network start — early on it answers with no eligible
      // masternodes and the connect fails. Retry instead of giving up: a
      // rejected connect must also not stay cached, or one warm-up hiccup
      // would poison every later proof verification.
      const deadline = Date.now() + 180000;

      for (;;) {
        const sdk = new evo.EvoSDK({
          network: 'local',
          trusted: true,
          quorumUrl: getQuorumListUrl(quorumListConfig),
          addresses: [address],
          proofs: true,
        });

        try {
          await sdk.connect();

          return { evo, sdk };
        } catch (error) {
          if (Date.now() >= deadline) {
            evoSdkCache.delete(address);
            throw error;
          }

          await wait(5000);
        }
      }
    })());
  }

  return evoSdkCache.get(address);
}

/**
 * Drop every cached EvoSDK. Call between scenarios that restart nodes, so a
 * later scenario cannot read through a connection pinned to a dead container.
 *
 * @return {void}
 */
export function resetEvoSdkCache() {
  evoSdkCache.clear();
}

/**
 * Read the error's kind without assuming it survives the WASM boundary.
 *
 * @param {*} error
 * @return {string|undefined}
 */
function readErrorName(error) {
  try {
    return error && error.name;
  } catch {
    // A WASM error object whose memory is already released throws on access.
    return undefined;
  }
}

/**
 * Create the `IPlatformProofVerifier` that `Dash.Client` requires before it
 * will broadcast anything.
 *
 * Mirrors the platform-test-suite verifier: an execution proof is demanded
 * wherever the transition family can produce one, and the families that cannot
 * fall back to a height-pinned snapshot of the affected state.
 *
 * @param {Config} config - node to verify against
 * @param {Config} quorumListConfig
 * @return {Object}
 */
export function createPlatformProofVerifier(config, quorumListConfig) {
  // The WASM SDK's default per-request timeout is 10 seconds, but waiting for
  // a state transition result must span at least one block interval — and the
  // local network produces empty blocks every 30 seconds. Left at the default,
  // every wait can exhaust its retries before a block was even due.
  //
  // `waitTimeoutMs` must NOT be set here: it makes rs-sdk wrap the wait in
  // `tokio::time::timeout`, which reads `std::time::Instant` — unimplemented
  // on wasm32 — and the whole module panics with `RuntimeError: unreachable`.
  // The per-request `timeoutMs` bounds each attempt through the wasm-safe
  // transport path instead, so the overall wait is still finite
  // (retries x timeoutMs).
  const waitSettings = {
    timeoutMs: 120000,
    retries: 2,
  };

  return {
    async verifyStateTransitionResult({ serializedStateTransition }) {
      const { evo, sdk } = await getEvoSdk(config, quorumListConfig);

      const stateTransition = evo.StateTransition.fromBytes(
        new Uint8Array(serializedStateTransition),
      );

      try {
        await sdk.stateTransitions.waitForResponse(stateTransition, waitSettings);
      } catch (error) {
        if (readErrorName(error) !== EXECUTION_NOT_PROVED) {
          throw error;
        }

        await sdk.stateTransitions.waitForAffectedState(stateTransition, waitSettings);
      }
    },

    async verifyDataContractHistory({ contractId, startAtMs, limit }) {
      const { sdk } = await getEvoSdk(config, quorumListConfig);

      const history = await sdk.contracts.getHistory({
        dataContractId: new Uint8Array(contractId),
        limit,
        startAtMs: Number(startAtMs),
      });

      return Array.from(history.entries()).map(([date, contract]) => ({
        date: BigInt(date),
        value: contract.toBytes(),
      }));
    },
  };
}

/**
 * Create a `Dash.Client` with a fresh, empty wallet.
 *
 * `skipSyncBeforeHeight` matters more than it looks: a local network has
 * already mined thousands of blocks registering masternodes by the time this
 * runs, and a wallet that scans all of them for a key created seconds ago
 * spends minutes finding nothing. Starting the transaction scan at the current
 * tip is what the platform-test-suite does for the same reason.
 *
 * @param {Config} config - node to talk to
 * @param {Config} quorumListConfig
 * @param {Object} [options]
 * @param {number} [options.skipSyncBeforeHeight]
 * @return {Client}
 */
export function createClient(config, quorumListConfig, { skipSyncBeforeHeight } = {}) {
  const wallet = {
    mnemonic: null,
    waitForInstantLockTimeout: 120000,
  };

  if (skipSyncBeforeHeight) {
    wallet.unsafeOptions = {
      skipSynchronizationBeforeHeight: skipSyncBeforeHeight,
    };
  }

  return new Dash.Client({
    network: 'regtest',
    dapiAddresses: [getDapiAddress(config)],
    platformProofVerifier: createPlatformProofVerifier(config, quorumListConfig),
    // Per-request gRPC deadline. The 10s default is calibrated for reads;
    // broadcasting an identity registration makes Tenderdash run CheckTx,
    // which verifies the asset lock's InstantSend signature, and on a local
    // network that can outlast 10s — the client then times out and retries a
    // broadcast that was never rejected.
    timeout: 60000,
    wallet,
  });
}

/**
 * Current Core block height.
 *
 * @param {CoreService} coreService
 * @return {Promise<number>}
 */
export async function getCoreHeight(coreService) {
  const { result } = await coreService.getRpcClient().getBlockCount();

  return result;
}

/**
 * Fund a client's wallet straight from the seed node's Core wallet.
 *
 * The platform-test-suite funds through a second wallet-lib wallet holding the
 * faucet key, but that wallet has to rediscover a coinbase output that was
 * mined before it existed, over a DAPI whose validators carry no address
 * index. Paying the client's address from Core instead means the only thing
 * the wallet has to see is a transaction that arrives while it is already
 * listening, which is the path wallet-lib is reliable on.
 *
 * @param {CoreService} coreService - Core of the seed node
 * @param {Client} client
 * @param {number} amount - duffs to send
 * @param {Object} [options]
 * @param {number} [options.timeoutMs]
 * @param {function(string): void} [options.log]
 * @return {Promise<{ address: string, balance: number }>}
 */
export async function fundClientFromCore(coreService, client, amount, {
  timeoutMs = 600000,
  log = () => {},
} = {}) {
  const account = await client.getWalletAccount();
  const { address } = account.getAddress();

  const rpcClient = coreService.getRpcClient();

  // sendToAddress takes DASH, and the wallet reports duffs
  const { result: transactionId } = await rpcClient.sendToAddress(address, amount / 1e8);

  log(`sent ${amount} duffs to ${address} in ${transactionId}`);

  // The payment must be observable on the Core it was sent through before the
  // wallet's stream can be blamed for not delivering it. A broadcast that
  // silently never made it into the mempool used to surface here as a generic
  // wallet-sync timeout minutes later, pointing every investigation at DAPI.
  const { result: mempool } = await rpcClient.getRawMemPool();

  if (!mempool.includes(transactionId)) {
    throw new Error(
      `funding transaction ${transactionId} did not enter the Core mempool after sendToAddress`,
    );
  }

  const privateKey = new PrivateKey();
  const throwawayAddress = privateKey.toAddress('regtest').toString();

  // Confirm the payment and verify it, rather than assuming two mined blocks
  // did the job.
  await rpcClient.generateToAddress(2, throwawayAddress, 10000000);

  const confirmDeadline = Date.now() + 60000;
  let confirmations = 0;

  while (Date.now() < confirmDeadline) {
    const { result: fundingTx } = await rpcClient.getTransaction(transactionId);

    confirmations = fundingTx.confirmations || 0;

    if (confirmations > 0) {
      log(`funding transaction confirmed in block ${fundingTx.blockheight}`
        + ` (${confirmations} confirmations)`);
      break;
    }

    await rpcClient.generateToAddress(1, throwawayAddress, 10000000);
    await wait(2000);
  }

  if (confirmations === 0) {
    throw new Error(
      `funding transaction ${transactionId} entered the mempool but was not mined within 60s`,
    );
  }

  const deadline = Date.now() + timeoutMs;

  let balance = 0;
  let polls = 0;

  while (Date.now() < deadline) {
    balance = account.getTotalBalance();

    if (balance >= amount) {
      return { address, balance };
    }

    polls += 1;

    if (polls % 10 === 0) {
      log(`waiting for wallet ${address}: ${balance} of ${amount} duffs`);

      // Nudge the chain occasionally: each new block re-triggers the wallet's
      // stream processing without flooding it. Mining on every poll would give
      // the sync more blocks to catch up on than it gains.
      await rpcClient.generateToAddress(1, throwawayAddress, 10000000);
    }

    await wait(3000);
  }

  // The chain-side facts are known good at this point, so say so: this
  // failure is in the wallet's transaction stream, not in the funding.
  throw new Error(
    `funding transaction ${transactionId} is confirmed on chain, but the wallet at ${address}`
      + ` only saw ${balance} of ${amount} duffs within ${timeoutMs}ms`,
  );
}

/**
 * A CoreService wrapping a node's already-running Core container.
 *
 * @param {Object} diContainer - awilix DI container
 * @param {Config} config
 * @return {Promise<CoreService>}
 */
export async function getRunningCoreService(diContainer, config) {
  const createRpcClient = diContainer.resolve('createRpcClient');
  const getConnectionHost = diContainer.resolve('getConnectionHost');
  const dockerCompose = diContainer.resolve('dockerCompose');
  const docker = diContainer.resolve('docker');

  const [containerId] = await dockerCompose.getContainerIds(config, {
    filterServiceNames: 'core',
  });

  if (!containerId) {
    throw new Error(`Core is not running on ${config.getName()}`);
  }

  const rpcClient = createRpcClient({
    port: config.get('core.rpc.port'),
    user: 'dashmate',
    pass: config.get('core.rpc.users.dashmate.password'),
    host: await getConnectionHost(config, 'core', 'core.rpc.host'),
  });

  return new CoreService(config, rpcClient, docker.getContainer(containerId));
}

/**
 * Sporks the local network turns on during setup.
 *
 * A node that joins later never learns them: setup activates them once on the
 * seed while every node of the group is already connected, and a Core that
 * finishes its masternode sync afterwards does not go back for them. Without
 * SPORK_19 in particular the joiner treats ChainLocks as disabled, never
 * obtains one, and drive-abci waits for a chain lock forever — so Tenderdash
 * never finishes its ABCI handshake and state sync cannot even begin.
 *
 * @type {string[]}
 */
const LOCAL_NETWORK_SPORKS = [
  'SPORK_2_INSTANTSEND_ENABLED',
  'SPORK_3_INSTANTSEND_BLOCK_FILTERING',
  'SPORK_9_SUPERBLOCKS_ENABLED',
  'SPORK_17_QUORUM_DKG_ENABLED',
  'SPORK_19_CHAINLOCKS_ENABLED',
];

/**
 * Activate the local network's sporks on a node that joined after setup.
 *
 * The node's config carries the group's spork private key, so it can sign the
 * spork messages itself.
 *
 * @param {Object} diContainer
 * @param {Config} config
 * @return {Promise<string[]>} the sporks activated
 */
export async function activateLocalSporks(diContainer, config) {
  const activateCoreSpork = diContainer.resolve('activateCoreSpork');

  const coreService = await getRunningCoreService(diContainer, config);

  for (const spork of LOCAL_NETWORK_SPORKS) {
    await activateCoreSpork(coreService.getRpcClient(), spork);
  }

  return LOCAL_NETWORK_SPORKS;
}

/**
 * Mine coins to a fresh address on the seed node and hand back its key.
 *
 * Reuses dashmate's own `wallet mint` task rather than shelling out to the
 * CLI, so the isolated home dir and per-suite ports are honoured.
 *
 * The task would otherwise start its own Core service, which fails once the
 * network is up ("Service core is already running"). Handing it a CoreService
 * wrapping the seed's running container makes it mine through that instead,
 * and also stops it from tearing the container down afterwards.
 *
 * @param {Object} diContainer - awilix DI container
 * @param {Config} seedConfig - the `local_seed` config
 * @param {number} amount - dash to mine
 * @return {Promise<{ address: string, privateKey: string, coreService: CoreService }>}
 */
export async function mintToNewAddress(diContainer, seedConfig, amount) {
  const generateToAddressTask = diContainer.resolve('generateToAddressTask');

  const coreService = await getRunningCoreService(diContainer, seedConfig);

  const context = await generateToAddressTask(seedConfig, amount).run({
    coreService,
    address: null,
    network: seedConfig.get('network'),
  });

  if (!context.privateKey) {
    throw new Error('dashmate wallet mint did not return a private key');
  }

  return {
    address: context.address,
    privateKey: context.privateKey,
    coreService,
  };
}
