import Dash from 'dash';

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

      const sdk = new evo.EvoSDK({
        network: 'local',
        trusted: true,
        quorumUrl: getQuorumListUrl(quorumListConfig),
        addresses: [address],
        proofs: true,
      });

      await sdk.connect();

      return { evo, sdk };
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
  return {
    async verifyStateTransitionResult({ serializedStateTransition }) {
      const { evo, sdk } = await getEvoSdk(config, quorumListConfig);

      const stateTransition = evo.StateTransition.fromBytes(
        new Uint8Array(serializedStateTransition),
      );

      try {
        await sdk.stateTransitions.waitForResponse(stateTransition);
      } catch (error) {
        if (readErrorName(error) !== EXECUTION_NOT_PROVED) {
          throw error;
        }

        await sdk.stateTransitions.waitForAffectedState(stateTransition);
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
 * Create a `Dash.Client` whose wallet holds the faucet key, so it can fund
 * other wallets.
 *
 * @param {Config} config - node to talk to
 * @param {Config} quorumListConfig
 * @param {string} faucetPrivateKey - WIF private key holding mined coins
 * @return {Client}
 */
export function createFaucetClient(config, quorumListConfig, faucetPrivateKey) {
  return new Dash.Client({
    network: 'regtest',
    dapiAddresses: [getDapiAddress(config)],
    platformProofVerifier: createPlatformProofVerifier(config, quorumListConfig),
    wallet: {
      privateKey: faucetPrivateKey,
      waitForInstantLockTimeout: 120000,
    },
  });
}

/**
 * Create a `Dash.Client` with a fresh wallet funded from the faucet client.
 *
 * @param {Config} config - node to talk to
 * @param {Config} quorumListConfig
 * @param {Client} faucetClient
 * @param {number} amount - duffs to fund the new wallet with
 * @return {Promise<Client>}
 */
export async function createFundedClient(config, quorumListConfig, faucetClient, amount) {
  const { default: fundWallet } = await import('@dashevo/wallet-lib/src/utils/fundWallet.js');

  const client = new Dash.Client({
    network: 'regtest',
    dapiAddresses: [getDapiAddress(config)],
    platformProofVerifier: createPlatformProofVerifier(config, quorumListConfig),
    wallet: {
      mnemonic: null,
      waitForInstantLockTimeout: 120000,
    },
  });

  await fundWallet(faucetClient.wallet, client.wallet, amount);

  return client;
}

/**
 * Mine coins to a fresh address on the seed node and hand back its key.
 *
 * Reuses dashmate's own `wallet mint` task rather than shelling out to the
 * CLI, so the isolated home dir and per-suite ports are honoured.
 *
 * @param {Object} container - awilix DI container
 * @param {Config} seedConfig - the `local_seed` config
 * @param {number} amount - dash to mine
 * @return {Promise<{ address: string, privateKey: string }>}
 */
export async function mintToNewAddress(container, seedConfig, amount) {
  const generateToAddressTask = container.resolve('generateToAddressTask');

  const context = await generateToAddressTask(seedConfig, amount).run({
    address: null,
    network: seedConfig.get('network'),
  });

  if (!context.privateKey) {
    throw new Error('dashmate wallet mint did not return a private key');
  }

  return {
    address: context.address,
    privateKey: context.privateKey,
  };
}
