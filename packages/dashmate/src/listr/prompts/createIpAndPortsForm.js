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
  let initialIp;
  if (!options.skipInitial) {
    initialIp = await publicIp.v4();
  }

  // TODO User input fo IP only?
  const fields = [
    {
      name: 'ip',
      message: 'External IP address IPv4',
      initial: initialIp,
      validate: validateIPv4,
    },
  ];

  // TODO: we can't use mainnet ports for other networks

  if (network !== PRESET_MAINNET) {
    let initialCoreP2PPort;
    if (!options.skipInitial) {
      initialCoreP2PPort = systemConfigs[network].core.p2p.port.toString();
    }

    fields.push({
      name: 'coreP2PPort',
      message: 'Core P2P Port',
      initial: initialCoreP2PPort,
      validate: validatePort,
    });
  }

  if (options.isHPMN) {
    let initialPlatformP2PPort;
    if (!options.skipInitial) {
      initialPlatformP2PPort = systemConfigs[network].platform.drive.tenderdash.p2p.port.toString();
    }

    fields.push({
      name: 'platformP2PPort',
      message: 'Platform P2P port',
      initial: initialPlatformP2PPort,
      validate: validatePort,
    });

    let initialPlatformHTTPPort;
    if (!options.skipInitial) {
      initialPlatformHTTPPort = systemConfigs[network].platform.dapi.envoy.http.port.toString();
    }

    fields.push({
      name: 'platformHTTPPort',
      message: 'Platform HTTP port',
      initial: initialPlatformHTTPPort,
      validate: validatePort,
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
      if (network === PRESET_MAINNET) {
        return validateIPv4(ip);
      }

      if (options.isHPMN) {
        if (coreP2PPort === platformP2PPort
          || coreP2PPort === platformHTTPPort
          || platformP2PPort === platformHTTPPort) {
          return 'same ports are used';
        }
      }

      return validateIPv4(ip) && validatePort(coreP2PPort)
        && (!options.isHPMN || (validatePort(platformP2PPort) && validatePort(platformHTTPPort)));
    },
  };
}

module.exports = createIpAndPortsForm;
