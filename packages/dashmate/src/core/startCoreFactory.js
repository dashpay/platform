import CoreService from './CoreService.js';

/**
 * @param {createRpcClient} createRpcClient
 * @param {waitForCoreStart} waitForCoreStart
 * @param {waitForCoreSync} waitForCoreSync
 * @param {DockerCompose} dockerCompose
 * @param {getConnectionHost} getConnectionHost
 * @param {ensureFileMountExists} ensureFileMountExists
 * @return {startCore}
 */
export default function startCoreFactory(
  createRpcClient,
  waitForCoreStart,
  waitForCoreSync,
  dockerCompose,
  getConnectionHost,
  ensureFileMountExists,
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
    ensureFileMountExists(logFilePath, 0o666);

    const coreContainer = await dockerCompose.runService(
      config,
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
        user: 'dashmate',
        pass: config.get('core.rpc.users.dashmate.password'),
        host: await getConnectionHost(config, 'core', 'core.rpc.host'),
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
