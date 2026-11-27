import validatePort from './validatePort.js';
import { PRESET_MAINNET } from '../../../constants.js';

/**
 * Why a node's Core and Platform ports cannot be used together, if they cannot
 *
 * @param {Object} ports
 * @param {number} ports.core
 * @param {number} ports.p2p - Platform P2P
 * @param {number} ports.http - Platform HTTP
 * @return {string|null}
 */
export function findPortCollision({ core, p2p, http }) {
  if (p2p === http) {
    return `the Platform P2P and HTTP ports are both ${p2p}`;
  }

  if (core === p2p || core === http) {
    return `Core already uses port ${core}`;
  }

  return null;
}

/**
 * Port validators for a node on the given network
 *
 * Off mainnet, the ports mainnet uses by default are refused, so a testnet
 * node is never set up on them.
 *
 * @param {DefaultConfigs} defaultConfigs
 * @param {string} network
 * @return {{
 *   validateCoreP2PPort: function(string): boolean,
 *   validatePlatformP2PPort: function(string): (boolean|string),
 *   validatePlatformHTTPPort: function(string): (boolean|string),
 * }} each takes the port as typed
 */
export default function createPortValidators(defaultConfigs, network) {
  const mainnetConfig = defaultConfigs.get(PRESET_MAINNET);
  const mainnetCoreP2PPort = mainnetConfig.get('core.p2p.port').toString();
  const mainnetPlatformP2PPort = mainnetConfig.get('platform.drive.tenderdash.p2p.port').toString();

  function validateCoreP2PPort(value) {
    if (!validatePort(value)) {
      return false;
    }

    if (network !== PRESET_MAINNET && value === mainnetCoreP2PPort) {
      return false;
    }

    return true;
  }

  function validatePlatformP2PPort(value) {
    if (!validatePort(value)) {
      return false;
    }

    if (network !== PRESET_MAINNET && value === mainnetPlatformP2PPort) {
      return 'this port is reserved for mainnet';
    }

    return true;
  }

  function validatePlatformHTTPPort(value) {
    if (network !== PRESET_MAINNET && value === mainnetPlatformP2PPort) {
      return 'this port is reserved for mainnet';
    }

    return validatePort(value);
  }

  return { validateCoreP2PPort, validatePlatformP2PPort, validatePlatformHTTPPort };
}
