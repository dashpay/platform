const fs = require('fs');
const path = require('path');
const CoreService = require('./CoreService');

/**
 * @param {createRpcClient} createRpcClient
 * @param {waitForCoreStart} waitForCoreStart
 * @param {waitForCoreSync} waitForCoreSync
 * @param {DockerCompose} dockerCompose
 * @param {getConnectionHost} getConnectionHost
 * @return {startCore}
 */
function startCoreFactory(
  createRpcClient,
  waitForCoreStart,
  waitForCoreSync,
  dockerCompose,
  getConnectionHost,
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
        throw new Error('You cannot run masternode with wallet mode on');
      }

      coreCommand.push('--disablewallet=0');
    } else {
      coreCommand.push('--disablewallet=1');
    }

    const logFilePath = config.get('core.log.file.path');

    // Remove directory that could potentially be created by Docker mount
    if (fs.existsSync(logFilePath) && fs.lstatSync(logFilePath).isDirectory()) {
      fs.rmSync(logFilePath, { recursive: true });
    }

    if (!fs.existsSync(logFilePath)) {
      fs.mkdirSync(path.dirname(logFilePath), { recursive: true });
      fs.writeFileSync(logFilePath, '');
    }

    console.log(config.toEnvs())
    console.log(logFilePath)

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
        host: await getConnectionHost(config, 'core'),
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
