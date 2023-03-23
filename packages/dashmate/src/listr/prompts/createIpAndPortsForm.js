const publicIp = require('public-ip');

const systemConfigs = require('../../../configs/system');

const validateIPv4 = require('./validators/validateIPv4');
const validatePort = require('./validators/validatePort');

const {
  PRESET_MAINNET,
} = require('../../constants');

/**
 * @typedef {createIpAndPortsForm}
 * @param {string} network
 * @param {Object} [options]
 * @param {Object} [options.skipInitial=false]
 * @param {Object} [options.isHPMN=false]
 * @returns {Object}
 */
async function createIpAndPortsForm(network, options = {}) {
  const mainnetCfg = systemConfigs[PRESET_MAINNET];

  function validateCoreP2PPort(value) {
    if (network !== PRESET_MAINNET
      && value === mainnetCfg.core.p2p.port.toString()) {
      return false;
    }

    return validatePort(value);
  }

  function validatePlatformP2PPort(value) {
    if (network !== PRESET_MAINNET
      && value === mainnetCfg.platform.drive.tenderdash.p2p.port.toString()) {
      return 'this port is reserved for mainnet';
    }

    return validatePort(value);
  }

  function validatePlatformHTTPPort(value) {
    if (network !== PRESET_MAINNET
      && value === mainnetCfg.platform.drive.tenderdash.p2p.port.toString()) {
      return 'this port is reserved for mainnet';
    }

    return validatePort(value);
  }

  let initialIp;
  if (!options.skipInitial) {
    initialIp = await publicIp.v4();
  }

  let initialCoreP2PPort;
  if (!options.skipInitial || network === PRESET_MAINNET) {
    initialCoreP2PPort = mainnetCfg.core.p2p.port.toString();
  }

  const fields = [
    {
      name: 'ip',
      message: 'External IP address',
      hint: 'IPv4',
      initial: initialIp,
      validate: validateIPv4,
    },
    {
      name: 'coreP2PPort',
      message: 'Core P2P port',
      initial: initialCoreP2PPort,
      validate: validateCoreP2PPort,
      disabled: network === PRESET_MAINNET,
    },
  ];

  if (options.isHPMN) {
    let initialPlatformP2PPort;
    if (!options.skipInitial) {
      initialPlatformP2PPort = systemConfigs[network].platform.drive.tenderdash.p2p.port.toString();
    }

    fields.push({
      name: 'platformP2PPort',
      message: 'Platform P2P port',
      initial: initialPlatformP2PPort,
      validate: validatePlatformP2PPort,
      disabled: network === PRESET_MAINNET,
    });

    let initialPlatformHTTPPort;
    if (!options.skipInitial) {
      initialPlatformHTTPPort = systemConfigs[network].platform.dapi.envoy.http.port.toString();
    }

    fields.push({
      name: 'platformHTTPPort',
      message: 'Platform HTTP port',
      initial: initialPlatformHTTPPort,
      validate: validatePlatformHTTPPort,
      disabled: network === PRESET_MAINNET,
    });
  }

  return {
    type: 'form',
    name: 'ipAndPorts',
    header: `  Dashmate needs to collect your external public static IP address and port
  information to use in the registration transaction. You will need to ensure
  these ports are open and reachable from the public internet at this IP address
  in order to avoid PoSe bans.\n`,
    message: 'Enter IP address and ports:',
    choices: fields,
    validate: ({
      ip,
      coreP2PPort,
      platformP2PPort,
      platformHTTPPort,
    }) => {
      if (options.isHPMN) {
        if (coreP2PPort === platformP2PPort
          || coreP2PPort === platformHTTPPort
          || platformP2PPort === platformHTTPPort) {
          return 'same ports are used';
        }
      }

      return validateIPv4(ip) && validateCoreP2PPort(coreP2PPort)
        && (
          !options.isHPMN
          || (
            validatePlatformP2PPort(platformP2PPort)
            && validatePlatformHTTPPort(platformHTTPPort)
          )
        );
    },
  };
}

module.exports = createIpAndPortsForm;
