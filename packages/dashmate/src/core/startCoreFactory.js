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

    const isMasternode = config.get('core.masternode.enable');

    if (isMasternode) {
      // Check operatorPrivateKey is set (error will be thrown if not)
      config.get('core.masternode.operator.privateKey', true);
    }

    // Run Core service
    const coreCommand = [
      'dashd',
    ];

    if (options.addressIndex) {
      coreCommand.push('--addressindex=1');
    }

    if (options.wallet) {
      if (isMasternode) {
        throw new Error('You cannot run masternode with wallet mode on')
      }

      coreCommand.push('--disablewallet=0');
    } else {
      coreCommand.push('--disablewallet=1');
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
