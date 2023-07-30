const chalk = require('chalk');

function createPortIsNotReachableFormFactory(defaultConfigs, resolvePublicIpV4) {
  /**
   * @typedef {function} createIpAndPortsForm
   * @param {string | number} coreP2PPort
   * @returns {Object}
   */
  async function createPortIsNotReachableForm(coreP2PPort) {
    const externalIp = await resolvePublicIpV4() ?? 'unresolved';

    return {
      type: 'toggle',
      name: 'confirm',
      header: `You have chosen Core P2P port ${coreP2PPort}, `
        + 'however it looks not reachable on your host '
        + `${chalk.red(`(TCP ${externalIp}:${coreP2PPort} CLOSED)`)}`,
      message: 'Are you sure that you want to continue?',
      enabled: 'Yes',
      disabled: 'No',
    };
  }

  return createPortIsNotReachableForm;
}

module.exports = createPortIsNotReachableFormFactory;
