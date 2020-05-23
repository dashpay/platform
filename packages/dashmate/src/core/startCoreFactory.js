const CoreService = require('./CoreService');

/**
 * @param {createRpcClient} createRpcClient
 * @param {waitForCoreStart} waitForCoreStart
 * @param {waitForCoreSync} waitForCoreSync
 * @param {DockerCompose} dockerCompose
 * @return {startCore}
 */
function startCoreFactory(
  createRpcClient,
  waitForCoreStart,
  waitForCoreSync,
  dockerCompose,
) {
  /**
   * @typedef startCore
   * @param {string} preset
   * @param {Object} [options]
   * @param {boolean} [options.wallet=false]
   * @return {CoreService}
   */
  async function startCore(preset, options = {}) {
    // eslint-disable-next-line no-param-reassign
    options = { wallet: false, ...options };

    // Run Core service

    const coreCommand = [
      'dashd',
      '-conf=/dash/.dashcore/dash.conf',
      '-datadir=/dash/data',
      '-port=20001',
    ];

    if (options.wallet) {
      coreCommand.push('--disablewallet=0');
    }

    if (options.addressindex) {
      coreCommand.push('--addressindex=1');
    }

    const coreContainer = await dockerCompose.runService(
      preset,
      'core',
      coreCommand,
      [
        '--publish=20002:20002',
        '--detach',
      ],
    );

    const rpcClient = createRpcClient();

    const coreService = new CoreService(
      rpcClient,
      coreContainer,
    );

    // Wait Core to start

    await waitForCoreStart(coreService);

    return coreService;
  }

  return startCore;
}

module.exports = startCoreFactory;
