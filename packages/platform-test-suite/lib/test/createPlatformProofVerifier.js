const DAPIAddress = require('@dashevo/dapi-client/lib/dapiAddressProvider/DAPIAddress');

/**
 * `WasmSdkError.name` for a transition family whose proof cannot bind the
 * execution of one specific transition.
 *
 * @type {string}
 */
const EXECUTION_NOT_PROVED = 'ExecutionNotProved';

/**
 * Read the error's kind without assuming it survives the WASM boundary.
 *
 * @param {*} error
 * @returns {string|undefined}
 */
function readErrorName(error) {
  try {
    return error && error.name;
  } catch (readError) {
    // A WASM error object whose memory is already released throws on access.
    return undefined;
  }
}

/**
 * Convert a WASM SDK error into a plain `Error`.
 *
 * These are wasm-bindgen class instances rather than `Error`s, and they carry
 * their kind and message on prototype getters. Mocha runs the suite in
 * parallel workers and serializes a failure by copying the error's own
 * properties, so an unconverted one arrives with nothing in it and the run
 * reports a test that failed for no stated reason.
 *
 * @param {*} error
 * @returns {Error}
 */
function toReportableError(error) {
  if (error instanceof Error) {
    return error;
  }

  const name = readErrorName(error) || 'UnknownError';
  let message;
  try {
    message = (error && error.message) || String(error);
  } catch (readError) {
    message = 'error details are unavailable';
  }

  const reportable = new Error(`${name}: ${message}`);
  reportable.name = name;

  return reportable;
}

/**
 * Shared EvoSDK instance. One per process: the verifier is stateless and the
 * underlying WASM SDK multiplexes concurrent requests.
 *
 * @type {Promise<{ evo: Object, sdk: Object }>|null}
 */
let evoSdkPromise = null;

/**
 * Map a client/test-suite network name to an EvoSDK network name.
 *
 * @param {string} network
 * @returns {string}
 */
function normalizeNetwork(network) {
  if (network === 'regtest' || network === 'local') {
    return 'local';
  }

  if (network === 'testnet' || network === 'mainnet' || network === 'devnet') {
    return network;
  }

  throw new Error(`Unsupported network "${network}" for the platform proof verifier`);
}

/**
 * Map the test suite's NETWORK env to an EvoSDK network name.
 *
 * @returns {{ network: string, devnetName: (string|undefined) }}
 */
function resolveNetwork() {
  const network = normalizeNetwork(process.env.NETWORK || 'local');

  if (network === 'devnet') {
    const devnetName = process.env.EVO_SDK_DEVNET_NAME;
    if (!devnetName) {
      throw new Error(
        'NETWORK=devnet requires EVO_SDK_DEVNET_NAME so the proof verifier can locate the quorum endpoint',
      );
    }

    return { network: 'devnet', devnetName };
  }

  return { network, devnetName: undefined };
}

/**
 * Build EvoSDK masternode addresses from the same env vars the test-suite
 * clients use.
 *
 * @returns {string[]}
 */
function resolveAddresses() {
  const seeds = (process.env.DAPI_ADDRESSES || process.env.DAPI_SEED || '')
    .split(',')
    .map((seed) => seed.trim())
    .filter(Boolean);

  return seeds.map((seed) => {
    const address = new DAPIAddress(seed);

    if (
      address.isSelfSignedCertificateAllowed()
      && typeof process !== 'undefined'
      && process.release
      && process.release.name === 'node'
    ) {
      // The local network gateway serves a self-signed certificate. The WASM
      // SDK transport goes through fetch and has no per-request TLS escape
      // hatch. This is a test-only environment where quorum-verified proofs,
      // not TLS, are the trust boundary.
      process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0';
    }

    const protocol = address.getProtocol() === 'http' ? 'http' : 'https';

    return `${protocol}://${address.getHost()}:${address.getPort()}`;
  });
}

/**
 * Lazily create and connect the shared EvoSDK used for proof verification.
 *
 * @returns {Promise<{ evo: Object, sdk: Object }>}
 */
function getEvoSdk() {
  if (!evoSdkPromise) {
    evoSdkPromise = (async () => {
      // Dynamic import: @dashevo/evo-sdk is ESM-only and the test suite is CJS
      const evo = await import('@dashevo/evo-sdk');
      await evo.ensureInitialized();

      const { network, devnetName } = resolveNetwork();

      const sdk = new evo.EvoSDK({
        network,
        devnetName,
        trusted: true,
        addresses: resolveAddresses(),
      });

      await sdk.connect();

      return { evo, sdk, network };
    })();
  }

  return evoSdkPromise;
}

/**
 * Get the shared EvoSDK, failing closed if the caller's network differs from
 * the one the verifier is connected to: verifying against another network's
 * quorum set must throw, never silently pass.
 *
 * @param {string} callNetwork
 * @returns {Promise<{ evo: Object, sdk: Object }>}
 */
async function getEvoSdkForNetwork(callNetwork) {
  const { evo, sdk, network } = await getEvoSdk();

  if (callNetwork !== undefined && normalizeNetwork(callNetwork) !== network) {
    throw new Error(
      `Platform proof verifier is connected to "${network}" but the client requested verification for "${callNetwork}"`,
    );
  }

  return { evo, sdk };
}

/**
 * Create an `IPlatformProofVerifier` for `Dash.Client`, backed by the
 * Rust/WASM SDK. Its proved re-query authenticates an execution result or
 * affected-state snapshot end to end: GroveDB proof verification plus the
 * Tenderdash quorum signature over the root hash.
 *
 * Verification re-queries Platform through the WASM SDK's proved paths rather
 * than re-checking the exact bytes the JS transport received. Execution proof
 * is required wherever the transition family can produce one. The families
 * that cannot fall back to a height-pinned snapshot of the affected state,
 * which is not evidence that this exact transition executed; the caller
 * handles consensus errors from the original response before invoking this
 * verifier.
 *
 * @param {Object} [dependencies]
 * @param {Function} [dependencies.getEvoSdkForNetwork]
 * @returns {Object} IPlatformProofVerifier
 */
function createPlatformProofVerifier({
  getEvoSdkForNetwork: loadEvoSdkForNetwork = getEvoSdkForNetwork,
} = {}) {
  return {
    /**
     * @param {Object} input
     * @param {Uint8Array} input.serializedStateTransition
     * @param {string} input.network
     * @returns {Promise<void>}
     */
    async verifyStateTransitionResult({ serializedStateTransition, network }) {
      const { evo, sdk } = await loadEvoSdkForNetwork(network);

      const stateTransition = evo.StateTransition.fromBytes(
        new Uint8Array(serializedStateTransition),
      );

      try {
        await sdk.stateTransitions.waitForResponse(stateTransition);
      } catch (error) {
        // Balance top-ups, credit transfers and withdrawals, address funds
        // movements, shields and no-history token operations have no proof
        // that binds one specific transition, so the SDK reports that the
        // execution was not proved rather than that anything failed. Their
        // authenticated affected-state snapshot is the strongest result
        // available; every other family keeps failing closed here.
        if (readErrorName(error) !== EXECUTION_NOT_PROVED) {
          throw toReportableError(error);
        }

        try {
          await sdk.stateTransitions.waitForAffectedState(stateTransition);
        } catch (affectedStateError) {
          throw toReportableError(affectedStateError);
        }
      }
    },

    /**
     * @param {Object} input
     * @param {Uint8Array} input.contractId
     * @param {bigint} input.startAtMs
     * @param {number} input.limit
     * @param {number} input.offset
     * @param {string} input.network
     * @returns {Promise<Array<{ date: bigint, value: Uint8Array }>>}
     */
    async verifyDataContractHistory({
      contractId, startAtMs, limit, offset, network,
    }) {
      if (offset) {
        throw new Error(
          'The platform proof verifier does not support a non-zero data contract history offset',
        );
      }

      const { sdk } = await loadEvoSdkForNetwork(network);

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

module.exports = createPlatformProofVerifier;
// The shared EvoSDK is also what functional specs use to exercise
// WASM-SDK-only read surfaces (e.g. chained document queries) against
// the same network the verifier is bound to.
module.exports.getEvoSdkForNetwork = getEvoSdkForNetwork;
