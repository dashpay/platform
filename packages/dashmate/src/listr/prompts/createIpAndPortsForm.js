import { publicIpv4 } from 'public-ip';
import validatePort from './validators/validatePort.js';
import validateIPv4 from './validators/validateIPv4.js';
import { PRESET_MAINNET } from '../../constants.js';
import wait from '../../util/wait.js';

export default function createIpAndPortsFormFactory(defaultConfigs) {
  /**
   * @typedef {function} createIpAndPortsForm
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
    const mainnetCfg = defaultConfigs.get(PRESET_MAINNET);

    function validateCoreP2PPort(value) {
      if (!validatePort(value)) {
        return false;
      }

      if (network !== PRESET_MAINNET
        && value === mainnetCfg.get('core.p2p.port').toString()) {
        return false;
      }

      return true;
    }

    function validatePlatformP2PPort(value) {
      if (!validatePort(value)) {
        return false;
      }

      if (network !== PRESET_MAINNET
        && value === mainnetCfg.get('platform.drive.tenderdash.p2p.port').toString()) {
        return 'this port is reserved for mainnet';
      }

      return true;
    }

    function validatePlatformHTTPPort(value) {
      if (network !== PRESET_MAINNET
        && value === mainnetCfg.get('platform.drive.tenderdash.p2p.port').toString()) {
        return 'this port is reserved for mainnet';
      }

      return validatePort(value);
    }

    let { initialIp } = options;
    if (initialIp === null || initialIp === undefined) {
      initialIp = await Promise.race([
        publicIpv4().catch(() => ''),
        // Resolve in 10 seconds if public IP is not available
        wait(10000).then(() => ''),
      ]);
    }

    let { initialCoreP2PPort } = options;
    if (initialCoreP2PPort === undefined
      || initialCoreP2PPort === null
      || network === PRESET_MAINNET) {
      initialCoreP2PPort = defaultConfigs.get(network).get('core.p2p.port').toString();
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
      let { initialPlatformP2PPort } = options;
      if (initialPlatformP2PPort === null
        || initialPlatformP2PPort === undefined
        || network === PRESET_MAINNET) {
        initialPlatformP2PPort = defaultConfigs.get(network).get('platform.drive.tenderdash.p2p.port').toString();
      }

      fields.push({
        name: 'platformP2PPort',
        message: 'Platform P2P port',
        initial: initialPlatformP2PPort,
        validate: validatePlatformP2PPort,
        disabled: network === PRESET_MAINNET ? '(reserved for mainnet)' : false,
      });

      let { initialPlatformHTTPPort } = options;
      if (initialPlatformHTTPPort === null
        || initialPlatformHTTPPort === undefined
        || network === PRESET_MAINNET) {
        initialPlatformHTTPPort = defaultConfigs.get(network).get('platform.gateway.listeners.dapiAndDrive.port').toString();
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

  return createIpAndPortsForm;
}
