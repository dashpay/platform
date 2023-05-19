const publicIp = require('public-ip');

const systemConfigs = require('../../../configs/system');

const validateIPv4 = require('./validators/validateIPv4');
const validatePort = require('./validators/validatePort');

const {
  PRESET_MAINNET,
} = require('../../constants');

const wait = require('../../util/wait');

/**
 * @typedef {createIpAndPortsForm}
 * @param {string} network
 * @param {Object} [options]
 * @param {string} [options.initialIp]
 * @param {string} [options.initialCoreP2PPort]
 * @param {string} [options.initialPlatformP2PPort]
 * @param {string} [options.initialPlatformHTTPPort]
 * @param {Object} [options.isHPMN=false]
 * @returns {Object}
 */
async function createIpAndPortsForm(network, options = {}) {
  const mainnetCfg = systemConfigs[PRESET_MAINNET];

  function validateCoreP2PPort(value) {
    if (!validatePort(value)) {
      return false;
    }

    if (network !== PRESET_MAINNET
      && value === mainnetCfg.core.p2p.port.toString()) {
      return false;
    }

    return true;
  }

  function validatePlatformP2PPort(value) {
    if (!validatePort(value)) {
      return false;
    }

    if (network !== PRESET_MAINNET
      && value === mainnetCfg.platform.drive.tenderdash.p2p.port.toString()) {
      return 'this port is reserved for mainnet';
    }

    return true;
  }

  function validatePlatformHTTPPort(value) {
    if (network !== PRESET_MAINNET
      && value === mainnetCfg.platform.drive.tenderdash.p2p.port.toString()) {
      return 'this port is reserved for mainnet';
    }

    return validatePort(value);
  }

  let initialIp;
  if (options.initialIp !== '' || !options.initialIp) {
    initialIp = await Promise.race([
      publicIp.v4().catch(() => ''),
      // Resolve in 10 seconds if public IP is not available
      wait(10000).then(() => ''),
    ]);
  }

  let initialCoreP2PPort;
  if (options.initialCoreP2PPort !== '' || !options.initialCoreP2PPort || network === PRESET_MAINNET) {
    initialCoreP2PPort = systemConfigs[network].core.p2p.port.toString();
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
      disabled: network === PRESET_MAINNET ? '(reserved for mainnet)' : false,
    },
  ];

  if (options.isHPMN) {
    let initialPlatformP2PPort;
    if (options.initialPlatformP2PPort !== '' || !options.initialPlatformP2PPort || network === PRESET_MAINNET) {
      initialPlatformP2PPort = systemConfigs[network].platform.drive.tenderdash.p2p.port.toString();
    }

    fields.push({
      name: 'platformP2PPort',
      message: 'Platform P2P port',
      initial: initialPlatformP2PPort,
      validate: validatePlatformP2PPort,
      disabled: network === PRESET_MAINNET ? '(reserved for mainnet)' : false,
    });

    let initialPlatformHTTPPort;
    if (options.initialPlatformHTTPPort !== '' || !options.initialPlatformHTTPPort || network === PRESET_MAINNET) {
      initialPlatformHTTPPort = systemConfigs[network].platform.dapi.envoy.http.port.toString();
    }

    fields.push({
      name: 'platformHTTPPort',
      message: 'Platform HTTP port',
      initial: initialPlatformHTTPPort,
      validate: validatePlatformHTTPPort,
      disabled: network === PRESET_MAINNET ? '(reserved for mainnet)' : false,
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
      const areAllFieldsValid = validateIPv4(ip) && validateCoreP2PPort(coreP2PPort)
        && (
          !options.isHPMN
          || (
            validatePlatformP2PPort(platformP2PPort)
            && validatePlatformHTTPPort(platformHTTPPort)
          )
        );

      if (!areAllFieldsValid) {
        return false;
      }

      if (options.isHPMN) {
        if (coreP2PPort === platformP2PPort
          || coreP2PPort === platformHTTPPort
          || platformP2PPort === platformHTTPPort) {
          return 'same ports are used';
        }
      }

      return true;
    },
  };
}

module.exports = createIpAndPortsForm;
