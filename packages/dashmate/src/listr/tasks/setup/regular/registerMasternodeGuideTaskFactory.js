import { Listr } from 'listr2';
import deriveTenderdashNodeId from '../../../../tenderdash/deriveTenderdashNodeId.js';
import getConfigurationOutputFromContext from './getConfigurationOutputFromContext.js';

/**
 * @param {DefaultConfigs} defaultConfigs
 * @param {registerMasternodeWithCoreWallet} registerMasternodeWithCoreWallet
 * @param {registerMasternodeWithDMT} registerMasternodeWithDMT
 * @return {registerMasternodeGuideTask}
 */
export default function registerMasternodeGuideTaskFactory(
  defaultConfigs,
  registerMasternodeWithCoreWallet,
  registerMasternodeWithDMT,
) {
  /**
   * @typedef {registerMasternodeGuideTask}
   * @return {Listr}
   */
  async function registerMasternodeGuideTask() {
    const REGISTRARS = {
      CORE: 'dashCore',
      // TODO: Disabled until additional functionality like signing protx and so on is
      //  implemented there
      // ANDROID: 'dashAndroid',
      // IOS: 'dashIOS',
      DMT: 'dmt',
    };

    return new Listr([
      {
        title: 'Register masternode',
        task: async (ctx, task) => {
          const registrar = await task.prompt([
            {
              type: 'select',
              header: `  For security reasons, Dash masternodes should never store masternode owner or
  collateral private keys. Dashmate therefore cannot register a masternode for you
  directly. Instead, we will generate RPC commands that you can use in Dash Core
  or other external tools where keys are handled securely. During this process,
  dashmate can optionally generate configuration elements as necessary, such as
  the BLS operator key and the node id.

  Dash Masternode Tool (DMT) - Recommended for mainnet masternodes
                               so the collateral can be stored
                               on a hardware wallet for maximum security.

  Dash Core Wallet           - Recommended for testnet and devnet masternodes
                               where more flexibility is required.\n`,
              message: 'Which wallet will you use to store keys for your masternode?',
              choices: [
                { name: REGISTRARS.DMT, message: 'Dash Masternode Tool' },
                { name: REGISTRARS.CORE, message: 'Dash Core Wallet' },
              ],
              initial: REGISTRARS.DMT,
            },
          ]);

          // TODO: Refactor. It should be done as a separate tasks
          let state;
          if (registrar === REGISTRARS.CORE) {
            state = await registerMasternodeWithCoreWallet(ctx, task, defaultConfigs);
          } else if (registrar === REGISTRARS.DMT) {
            state = await registerMasternodeWithDMT(ctx, task);
          }

          ctx.config.set('core.masternode.operator.privateKey', state.operator.privateKey);

          ctx.config.set('externalIp', state.ipAndPorts.ip);
          ctx.config.set('core.p2p.port', state.ipAndPorts.coreP2PPort);

          if (ctx.isHP) {
            ctx.config.set('platform.drive.tenderdash.node.id', deriveTenderdashNodeId(state.platformNodeKey));
            ctx.config.set('platform.drive.tenderdash.node.key', state.platformNodeKey);

            ctx.config.set('platform.gateway.listeners.dapiAndDrive.port', state.ipAndPorts.platformHTTPPort);
            ctx.config.set('platform.drive.tenderdash.p2p.port', state.ipAndPorts.platformP2PPort);
          }

          // eslint-disable-next-line no-param-reassign
          task.output = await getConfigurationOutputFromContext(ctx);
        },
        options: {
          persistentOutput: true,
        },
      },
    ]);
  }

  return registerMasternodeGuideTask;
}
