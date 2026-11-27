import { isIPv4 } from 'net';
import getBLSPublicKeyFromPrivateKeyHex from './getBLSPublicKeyFromPrivateKeyHex.js';

// The ProTx version of registrations made with the legacy BLS scheme.
const LEGACY_BLS_VERSION = 1;

export const REGISTRATION_STATUS = {
  EVO: 'EVO',
  REGULAR: 'REGULAR',
  NOT_FOUND: 'NOT_FOUND',
  UNSUPPORTED_ENDPOINTS: 'UNSUPPORTED_ENDPOINTS',
  UNAVAILABLE: 'UNAVAILABLE',
};

/**
 * Re-encode a BLS public key from the basic scheme to the legacy one
 *
 * The two encodings differ only in the flag bits of the first byte: the basic
 * scheme sets the compression bit and keeps the sign in bit 5, the legacy one
 * keeps the sign in the top bit. Core shows a registration's operator key in
 * the scheme it was registered with. Read as a basic key, the legacy encoding
 * of a key is another valid key, so it may only be compared with legacy
 * registrations.
 *
 * @param {string} basicHex
 * @return {string}
 */
export function toLegacyBlsPublicKey(basicHex) {
  const bytes = Buffer.from(basicHex, 'hex');

  /* eslint-disable no-bitwise */
  bytes[0] = (bytes[0] & 0x1f) | ((bytes[0] & 0x20) !== 0 ? 0x80 : 0);
  /* eslint-enable no-bitwise */

  return bytes.toString('hex');
}

/**
 * @param {string} address - "host:port" or "[ipv6]:port"
 * @return {{ host: string, port: number }|null}
 */
function parseAddress(address) {
  const match = typeof address === 'string' && address.match(/^(?:\[([^\]]+)\]|([^:]+)):(\d+)$/);

  if (!match) {
    return null;
  }

  return { host: match[1] ?? match[2], port: Number(match[3]) };
}

/**
 * Find one registered Platform endpoint and check dashmate can serve it
 *
 * dashmate advertises Tenderdash at externalIp and issues the gateway
 * certificate for externalIp, so an endpoint on any other host, or a domain
 * name, cannot be served from this config.
 *
 * @param {Object} state - the registration's state
 * @param {string} addressesKey - key in state.addresses
 * @param {string} legacyPortKey - the legacy port field
 * @param {string} label
 * @param {string} externalIp
 * @return {{ port: number }|{ reason: string }}
 */
function readEndpoint(state, addressesKey, legacyPortKey, label, externalIp) {
  let address = state.addresses?.[addressesKey]?.[0];

  // Core before 23 has no `addresses` object: the host is the Core service's
  // and the port is in a field of its own.
  if (address === undefined) {
    const service = parseAddress(state.service);
    const port = state[legacyPortKey];

    if (!Number.isInteger(port) || port < 1 || port > 65535) {
      return { reason: `The registration has no ${label} port` };
    }

    address = `${service?.host ?? ''}:${port}`;
  }

  const parsed = parseAddress(address);

  if (!parsed || parsed.port < 1 || parsed.port > 65535) {
    return { reason: `The registered ${label} address '${address}' has no usable port` };
  }

  if (!isIPv4(parsed.host)) {
    return {
      reason: `The registered ${label} address ${address} is not an IPv4 address. dashmate serves`
        + ` Platform on the node's external IP ${externalIp}`,
    };
  }

  if (parsed.host !== externalIp) {
    return {
      reason: `The registered ${label} address ${address} is not on this node's external IP`
        + ` ${externalIp}. If the IP changed, update the registration with a ProUpServTx first`,
    };
  }

  return { port: parsed.port };
}

/**
 * Pick the registration of this node from the detailed registered list
 *
 * The entry is selected by the operator key, which Core keeps unique among
 * registered masternodes, so the answer belongs to this config's operator
 * rather than to whatever key the running Core uses. Each entry is compared in
 * the scheme it was registered with.
 *
 * @param {Object[]} protxList - `protx list registered true`
 * @param {Object} node
 * @param {string} node.operatorPublicKey - basic BLS encoding, hex
 * @param {string} node.externalIp
 * @return {Object}
 */
export function findPlatformRegistration(protxList, { operatorPublicKey, externalIp }) {
  if (!Array.isArray(protxList)) {
    return {
      status: REGISTRATION_STATUS.UNAVAILABLE,
      reason: 'Core returned an unexpected masternode list',
    };
  }

  const legacyOperatorPublicKey = toLegacyBlsPublicKey(operatorPublicKey);

  const entries = protxList.filter((item) => {
    const key = String(item?.state?.pubKeyOperator).toLowerCase();

    return item?.state?.version === LEGACY_BLS_VERSION
      ? key === legacyOperatorPublicKey
      : key === operatorPublicKey;
  });

  if (entries.length === 0) {
    return { status: REGISTRATION_STATUS.NOT_FOUND };
  }

  if (entries.length > 1) {
    return {
      status: REGISTRATION_STATUS.UNAVAILABLE,
      reason: `${entries.length} registrations use this node's operator key`,
    };
  }

  const [{ type, proTxHash, state }] = entries;

  if (type === 'Regular') {
    return { status: REGISTRATION_STATUS.REGULAR, proTxHash };
  }

  if (type !== 'Evo') {
    return {
      status: REGISTRATION_STATUS.UNAVAILABLE,
      reason: `Core reported an unknown masternode type '${type}'`,
    };
  }

  const p2p = readEndpoint(state, 'platform_p2p', 'platformP2PPort', 'Platform P2P', externalIp);
  const https = readEndpoint(state, 'platform_https', 'platformHTTPPort', 'Platform HTTPS', externalIp);

  const reason = p2p.reason ?? https.reason;

  if (reason) {
    return { status: REGISTRATION_STATUS.UNSUPPORTED_ENDPOINTS, proTxHash, reason };
  }

  return {
    status: REGISTRATION_STATUS.EVO,
    proTxHash,
    // Core prints the registered id as it was given (SetHex / ToString), which
    // is the lowercase hex dashmate derives.
    platformNodeId: String(state.platformNodeID).toLowerCase(),
    platformP2PPort: p2p.port,
    platformHttpPort: https.port,
    poSeBanHeight: state.PoSeBanHeight > 0 ? state.PoSeBanHeight : null,
  };
}

/**
 * Read this node's masternode registration from a synced Core
 *
 * `masternode status` is not used: Core fills it only once its active manager
 * accepts the node, so it says nothing for a PoSe-banned node, one waiting for
 * its ProTx, or one whose IP or operator key changed. The detailed registered
 * list has every registration, and is trusted only once Core has caught up,
 * because Core reads it at its own tip.
 *
 * @typedef {function} readPlatformRegistration
 * @param {RpcClient} rpcClient - with callWithParams (see createRpcClient)
 * @param {Object} node
 * @param {string} node.operatorPrivateKey
 * @param {string} node.externalIp
 * @return {Promise<Object>} status, and for a registration its proTxHash; for
 *  EVO also platformNodeId, platformP2PPort, platformHttpPort, poSeBanHeight;
 *  for UNAVAILABLE and UNSUPPORTED_ENDPOINTS a reason
 */
export default async function readPlatformRegistration(
  rpcClient,
  { operatorPrivateKey, externalIp },
) {
  const operatorPublicKey = await getBLSPublicKeyFromPrivateKeyHex(operatorPrivateKey);

  let protxList;
  try {
    const { result: mnsync } = await rpcClient.mnsync('status');
    const { result: chain } = await rpcClient.getBlockchainInfo();

    if (mnsync?.IsBlockchainSynced !== true || chain?.initialblockdownload !== false) {
      return {
        status: REGISTRATION_STATUS.UNAVAILABLE,
        reason: 'Core has not finished syncing the blockchain, so its masternode list may be out of date',
      };
    }

    // dashd-rpc would send `true` as a string.
    ({ result: protxList } = await rpcClient.callWithParams('protx', ['list', 'registered', true]));
  } catch (e) {
    return { status: REGISTRATION_STATUS.UNAVAILABLE, reason: `Core RPC failed: ${e.message}` };
  }

  return findPlatformRegistration(protxList, {
    operatorPublicKey: operatorPublicKey.toLowerCase(),
    externalIp,
  });
}
