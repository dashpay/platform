const publicIp = require('public-ip');

const { base } = require('../../../configs/system');

function createIpAndPortsFormFactory() {
  function validateIp(ip) {
    return Boolean(ip.match(/^((25[0-5]|(2[0-4]|1\d|[1-9]|)\d)\.?\b){4}$/));
  }

  function validatePort(port) {
    const portNumber = Math.floor(Number(port));

    return portNumber >= 1
      && portNumber <= 65535
      && portNumber.toString() === port;
  }

  /**
   * @typedef {createIpAndPortsForm}
   * @param {Object} [options]
   * @param {Object} [options.skipInitial=false]
   * @param {Object} [options.isHPMN=false]
   * @returns {Object}
   */
  async function createIpAndPortsForm(options = {}) {
    let initialIp;
    if (!options.skipInitial) {
      initialIp = await publicIp.v4();
    }

    let initialCoreP2PPort;
    if (!options.skipInitial) {
      initialCoreP2PPort = base.core.p2p.port.toString();
    }

    // TODO: Ports shouldn't be equal and must not be busy
    // TODO: Validate that IP address is available from outside
    // TODO: Validate ports according to the current network
    // TODO: We should take initial values from corresponding configs not from the base

    const fields = [
      {
        name: 'ip',
        message: 'External IP address IPv4',
        initial: initialIp,
        validate: validateIp,
      },
      {
        name: 'coreP2PPort',
        message: 'Core P2P Port',
        initial: initialCoreP2PPort,
        validate: validatePort,
      },
    ];

    if (options.isHPMN) {
      let initialPlatformP2PPort;
      if (!options.skipInitial) {
        initialPlatformP2PPort = base.platform.drive.tenderdash.p2p.port.toString();
      }

      fields.push({
        name: 'platformP2PPort',
        message: 'Platform P2P port',
        initial: initialPlatformP2PPort,
        validate: validatePort,
      });

      let initialPlatformHTTPPort;
      if (!options.skipInitial) {
        initialPlatformHTTPPort = base.platform.dapi.envoy.http.port.toString();
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
      header: 'The node external IP address must be static and will be used by the ..'
        + ' network ..\n',
      message: 'Enter IP address and ports:',
      choices: fields,
      validate: ({
        ip,
        coreP2PPort,
        platformP2PPort,
        platformHTTPPort,
      }) => {
        return validateIp(ip) && validatePort(coreP2PPort)
        && (!options.isHPMN || (validatePort(platformP2PPort) && validatePort(platformHTTPPort)))
      },
    };
  }

  return createIpAndPortsForm;
}

module.exports = createIpAndPortsFormFactory;
