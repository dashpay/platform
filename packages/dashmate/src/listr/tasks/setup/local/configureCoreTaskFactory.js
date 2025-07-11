import { Listr } from 'listr2';
import { Observable } from 'rxjs';
import DashCoreLib from '@dashevo/dashcore-lib';
import waitForNodesToHaveTheSameHeight from '../../../../core/waitForNodesToHaveTheSameHeight.js';

import { NETWORK_LOCAL, HPMN_COLLATERAL_AMOUNT } from '../../../../constants.js';

const { PrivateKey } = DashCoreLib;

/**
 * @param {writeConfigTemplates} writeConfigTemplates
 * @param {startCore} startCore
 * @param {generateBlocks} generateBlocks
 * @param {waitForCoreSync} waitForCoreSync
 * @param {activateCoreSpork} activateCoreSpork
 * @param {generateToAddressTask} generateToAddressTask
 * @param {registerMasternodeTask} registerMasternodeTask
 * @param {generateBlsKeys} generateBlsKeys
 * @param {enableMultiCoreQuorumsTask} enableMultiCoreQuorumsTask
 * @param {enableSingleCoreQuorumTask} enableSingleCoreQuorumTask
 * @param {waitForMasternodesSync} waitForMasternodesSync
 * @param {ConfigFile} configFile
 * @return {configureCoreTask}
 */
export default function configureCoreTaskFactory(
  writeConfigTemplates,
  startCore,
  generateBlocks,
  waitForCoreSync,
  activateCoreSpork,
  generateToAddressTask,
  registerMasternodeTask,
  generateBlsKeys,
  enableMultiCoreQuorumsTask,
  enableSingleCoreQuorumTask,
  waitForMasternodesSync,
  configFile,
) {
  const WAIT_FOR_NODES_TIMEOUT = 60 * 5 * 1000;

  /**
   * @typedef {configureCoreTask}
   * @param {Config[]} configGroup
   * @return {Listr}
   */
  function configureCoreTask(configGroup) {
    return new Listr([
      {
        task: async (ctx) => {
          const network = configGroup[0].get('network');
          const sporkPrivKey = new PrivateKey(undefined, network);
          const sporkAddress = sporkPrivKey.toAddress(network).toString();

          const seedNodes = configGroup.filter((config) => config.getName() === 'local_seed')
            .map((config) => ({
              host: config.get('externalIp'),
              port: config.get('core.p2p.port'),
            }));

          configGroup.forEach((config) => {
            // Set seeds
            if (config.getName() !== 'local_seed') {
              config.set(
                'core.p2p.seeds',
                seedNodes,
              );
            }

            // Set sporks key
            config.set(
              'core.spork.address',
              sporkAddress,
            );

            config.set(
              'core.spork.privateKey',
              sporkPrivKey.toWIF(),
            );

            // Write configs
            writeConfigTemplates(config);
          });

          return new Listr([
            {
              title: 'Starting seed node as a wallet',
              task: async () => {
                const config = configGroup.find((c) => c.getName() === 'local_seed');

                ctx.coreService = await startCore(
                  config,
                  { wallet: true, addressIndex: true },
                );
              },
            },
            {
              title: 'Create wallet',
              task: async () => {
                const disablePrivateKeys = false;
                const createBlankWallet = false;
                const walletPassphrase = '';
                const avoidReuse = false;
                const loadOnStartup = true;
                const descriptors = false;

                await ctx.coreService.getRpcClient().createWallet(
                  'main',
                  disablePrivateKeys,
                  createBlankWallet,
                  walletPassphrase,
                  avoidReuse,
                  descriptors,
                  loadOnStartup,
                );
              },
            },
            {
              title: 'Generating funds to use as a collateral for masternodes',
              task: () => {
                // Generate for evnodes (- a seed node) + some cash for fees
                const amount = HPMN_COLLATERAL_AMOUNT * (configGroup.length - 1) + 100;
                return generateToAddressTask(
                  configGroup.find((c) => c.getName() === 'local_seed'),
                  amount,
                );
              },
            },
            {
              title: 'Activating v19 and v20',
              task: () => new Observable(async (observer) => {
                const activationHeight = 901;
                const blocksToGenerateInOneStep = 10;

                let blocksGenerated = 0;
                let currentBlockHeight = 0;

                do {
                  await generateBlocks(
                    ctx.coreService,
                    blocksToGenerateInOneStep,
                    NETWORK_LOCAL,
                    // eslint-disable-next-line no-loop-func
                    (blocks) => {
                      blocksGenerated += blocks;

                      observer.next(`${blocksGenerated} blocks generated`);
                    },
                  );

                  ({
                    result: currentBlockHeight,
                  } = await ctx.coreService.getRpcClient().getBlockCount());
                } while (activationHeight > currentBlockHeight);

                observer.complete();

                return this;
              }),
            },
            {
              title: 'Register masternodes',
              task: async () => {
                const masternodeConfigs = configGroup.filter((config) => config.get('core.masternode.enable'));

                const subTasks = masternodeConfigs.map((config, index) => ({
                  title: `Register ${config.getName()} masternode`,
                  task: () => new Listr([
                    {
                      title: 'Generate a masternode operator key',
                      task: async (task) => {
                        ctx.operator = await generateBlsKeys();

                        config.set('core.masternode.operator.privateKey', ctx.operator.privateKey);

                        configFile.markAsChanged();

                        // Write configs
                        writeConfigTemplates(config);

                        // eslint-disable-next-line no-param-reassign
                        task.output = `Public key: ${ctx.operator.publicKey}\nPrivate key: ${ctx.operator.privateKey}`;
                      },
                      options: { persistentOutput: true },
                    },
                    {
                      // first masternode has 10% operatorReward
                      task: () => registerMasternodeTask(config, true, index === 0 ? '10.00' : '0.00'),
                    },
                  ]),
                }));

                // eslint-disable-next-line consistent-return
                return new Listr(subTasks);
              },
            },
            {
              title: 'Stopping wallet',
              task: async () => {
                await ctx.coreService.stop();
              },
            },
            {
              title: 'Starting nodes',
              task: async () => {
                ctx.coreServices = await Promise.all(
                  configGroup.map((config) => startCore(config)),
                );

                ctx.rpcClients = ctx.coreServices.map((coreService) => coreService.getRpcClient());

                ctx.seedCoreService = ctx.coreServices.find((coreService) => (
                  coreService.getConfig().getName() === 'local_seed'
                ));

                ctx.seedRpcClient = ctx.seedCoreService.getRpcClient();
              },
            },
            {
              title: 'Force masternodes to sync',
              task: async () => {
                await Promise.all(ctx.coreServices.map((coreService) => (
                  // TODO: Rename function "wait -> force"
                  waitForMasternodesSync(coreService.getRpcClient())
                )));
              },
            },
            {
              title: 'Wait for nodes to have the same height',
              task: () => waitForNodesToHaveTheSameHeight(
                ctx.rpcClients,
                WAIT_FOR_NODES_TIMEOUT,
              ),
            },
            {
              title: 'Enable sporks',
              task: async () => {
                const sporks = [
                  'SPORK_2_INSTANTSEND_ENABLED',
                  'SPORK_3_INSTANTSEND_BLOCK_FILTERING',
                  'SPORK_9_SUPERBLOCKS_ENABLED',
                  'SPORK_17_QUORUM_DKG_ENABLED',
                  'SPORK_19_CHAINLOCKS_ENABLED',
                ];

                await Promise.all(
                  sporks.map(async (spork) => (
                    activateCoreSpork(ctx.seedCoreService.getRpcClient(), spork))),
                );
              },
            },
            {
              title: 'Wait for nodes to have the same height',
              task: () => waitForNodesToHaveTheSameHeight(
                ctx.rpcClients,
                WAIT_FOR_NODES_TIMEOUT,
              ),
            },
            {
              title: 'Make sure masternodes are enabled',
              task: async () => {
                const { result: masternodesStatus } = await ctx.seedRpcClient.masternodelist('status');

                const hasNotEnabled = Boolean(
                  Object.values(masternodesStatus)
                    .find((status) => status !== 'ENABLED'),
                );

                if (hasNotEnabled) {
                  throw new Error('Not all masternodes are enabled');
                }
              },
            },
            {
              title: 'Wait for quorum to be enabled',
              enabled: configGroup.length - 1 === 1,
              task: () => enableSingleCoreQuorumTask(),
            },
            {
              title: 'Wait for quorums to be enabled',
              enabled: configGroup.length - 1 > 1,
              task: () => enableMultiCoreQuorumsTask(),
            },
            {
              title: 'Wait for nodes to have the same height',
              task: () => waitForNodesToHaveTheSameHeight(
                ctx.rpcClients,
                WAIT_FOR_NODES_TIMEOUT,
              ),
            },
            {
              title: 'Activating v21 fork',
              task: () => new Observable(async (observer) => {
                // Drive expect all quorums available when we activate mn_rr (activation of
                // Evolution)
                // We activate v21 at block 1000 when we expect all quorums already formed
                const activationHeight = 1001;
                const blocksToGenerateInOneStep = 10;

                let blocksGenerated = 0;
                let currentBlockHeight = 0;

                do {
                  await generateBlocks(
                    ctx.coreService,
                    blocksToGenerateInOneStep,
                    NETWORK_LOCAL,
                    // eslint-disable-next-line no-loop-func
                    (blocks) => {
                      blocksGenerated += blocks;

                      observer.next(`${blocksGenerated} blocks generated`);
                    },
                  );

                  ({
                    result: currentBlockHeight,
                  } = await ctx.coreService.getRpcClient().getBlockCount());
                } while (activationHeight > currentBlockHeight);

                observer.complete();

                return this;
              }),
            },
            {
              title: 'Wait for nodes to have the same height',
              task: () => waitForNodesToHaveTheSameHeight(
                ctx.rpcClients,
                WAIT_FOR_NODES_TIMEOUT,
              ),
            },
            {
              title: 'Stopping nodes',
              task: async () => (Promise.all(
                ctx.coreServices.map((coreService) => coreService.stop()),
              )),
            },
          ]);
        },
      },
    ]);
  }

  return configureCoreTask;
}
