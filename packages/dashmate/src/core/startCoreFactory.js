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
   * @param {Config} config
   * @param {Object} [options]
   * @param {boolean} [options.wallet=false]
   * @param {boolean} [options.addressIndex=false]
   * @return {CoreService}
   */
  async function startCore(config, options = {}) {
    // eslint-disable-next-line no-param-reassign
    options = {
      wallet: false,
      addressIndex: false,
      ...options,
    };

    // Run Core service

    const coreCommand = [
      'dashd',
    ];

    const isMasternode = config.get('core.masternode.enable');

    if (isMasternode) {
      // Check operatorPrivateKey is set
      config.get('core.masternode.operator.privateKey', true);
    }

    if (options.addressIndex) {
      coreCommand.push('--addressindex=1');
    }

    if (options.wallet) {
      config.set('core.masternode.enable', 0)
      config.set('core.masternode.operator.privateKey', null)

      coreCommand.push('--disablewallet=0');
    }

    const coreContainer = await dockerCompose.runService(
      config.toEnvs(),
      'core',
      coreCommand,
      [
        '--service-ports',
        '--detach',
      ],
    );

    const rpcClient = createRpcClient(
      {
        port: config.get('core.rpc.port'),
        user: config.get('core.rpc.user'),
        pass: config.get('core.rpc.password'),
      },
    );

    const coreService = new CoreService(
      config,
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
