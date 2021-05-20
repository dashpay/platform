const { Listr } = require('listr2');
const { Observable } = require('rxjs');

const {
  PrivateKey,
} = require('@dashevo/dashcore-lib');

const waitForNodesToHaveTheSameSporks = require('../../../../core/waitForNodesToHaveTheSameSporks');
const waitForNodesToHaveTheSameHeight = require('../../../../core/waitForNodesToHaveTheSameHeight');

const { NETWORK_LOCAL, MASTERNODE_DASH_AMOUNT } = require('../../../../constants');

/**
 * @param {renderServiceTemplates} renderServiceTemplates
 * @param {writeServiceConfigs} writeServiceConfigs
 * @param {startCore} startCore
 * @param {generateBlocks} generateBlocks
 * @param {waitForCoreSync} waitForCoreSync
 * @param {activateCoreSpork} activateCoreSpork
 * @param {generateToAddressTask} generateToAddressTask
 * @param {registerMasternodeTask} registerMasternodeTask
 * @param {generateBlsKeys} generateBlsKeys
 * @param {enableCoreQuorumsTask} enableCoreQuorumsTask
 * @param {waitForMasternodesSync} waitForMasternodesSync
 * @return {configureCoreTask}
 */
function configureCoreTaskFactory(
  renderServiceTemplates,
  writeServiceConfigs,
  startCore,
  generateBlocks,
  waitForCoreSync,
  activateCoreSpork,
  generateToAddressTask,
  registerMasternodeTask,
  generateBlsKeys,
  enableCoreQuorumsTask,
  waitForMasternodesSync,
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
            const configFiles = renderServiceTemplates(config);
            writeServiceConfigs(config.getName(), configFiles);
          });

          return new Listr([
            {
              title: 'Starting seed node as a wallet',
              task: async () => {
                const config = configGroup.find((c) => c.getName() === 'local_seed');

                ctx.coreService = await startCore(config, { wallet: true, addressIndex: true });
              },
            },
            {
              title: 'Activating DIP3',
              task: () => new Observable(async (observer) => {
                const dip3ActivationHeight = 500;
                const blocksToGenerateInOneStep = 10;

                let blocksGenerated = 0;
                let {
                  result: currentBlockHeight,
                } = await ctx.coreService.getRpcClient().getBlockCount();

                do {
                  ({
                    result: currentBlockHeight,
                  } = await ctx.coreService.getRpcClient().getBlockCount());

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
                } while (dip3ActivationHeight > currentBlockHeight);

                observer.complete();

                return this;
              }),
            },
            {
              title: 'Generating funds to use as a collateral for masternodes',
              task: () => {
                const amount = MASTERNODE_DASH_AMOUNT * configGroup.length;
                return generateToAddressTask(
                  configGroup.find((c) => c.getName() === 'local_seed'),
                  amount,
                );
              },
            },
            {
              title: 'Register masternodes',
              task: async () => {
                const masternodeConfigs = configGroup.filter((config) => config.get('core.masternode.enable'));

                const subTasks = masternodeConfigs.map((config) => ({
                  title: `Register ${config.getName()} masternode`,
                  skip: () => {
                    if (config.get('core.masternode.operator.privateKey')) {
                      return `Masternode operator private key ('core.masternode.operator.privateKey') is already set in ${config.getName()} config`;
                    }

                    return false;
                  },
                  task: () => new Listr([
                    {
                      title: 'Generate a masternode operator key',
                      task: async (task) => {
                        ctx.operator = await generateBlsKeys();

                        config.set('core.masternode.operator.privateKey', ctx.operator.privateKey);

                        // eslint-disable-next-line no-param-reassign
                        task.output = `Public key: ${ctx.operator.publicKey}\nPrivate key: ${ctx.operator.privateKey}`;
                      },
                      options: { persistentOutput: true },
                    },
                    {
                      task: () => registerMasternodeTask(config),
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

                ctx.mockTime = 0;
                ctx.bumpMockTime = async (time = 1) => {
                  ctx.mockTime += time;

                  await Promise.all(
                    ctx.rpcClients.map((rpcClient) => rpcClient.setMockTime(ctx.mockTime)),
                  );
                };
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
              title: 'Set initial mock time',
              task: async () => {
                // Set initial mock time from the last block
                const { result: bestBlockHash } = await ctx.seedRpcClient.getBestBlockHash();
                const { result: bestBlock } = await ctx.seedRpcClient.getBlock(bestBlockHash);

                await ctx.bumpMockTime(bestBlock.time);

                // Sync nodes
                await ctx.bumpMockTime();

                await generateBlocks(
                  ctx.seedCoreService,
                  1,
                  NETWORK_LOCAL,
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
              title: 'Wait for nodes to have the same sporks',
              task: () => waitForNodesToHaveTheSameSporks(ctx.coreServices),
            },
            {
              title: 'Activating DIP8 to enable ChainLocks',
              task: () => new Observable(async (observer) => {
                let isDip8Activated = false;
                let blockchainInfo;

                let blocksGenerated = 0;

                const blocksToGenerateInOneStep = 10;

                do {
                  ({
                    result: blockchainInfo,
                  } = await ctx.seedCoreService.getRpcClient().getBlockchainInfo());

                  isDip8Activated = blockchainInfo.bip9_softforks.dip0008.status === 'active';

                  if (isDip8Activated) {
                    break;
                  }

                  await generateBlocks(
                    ctx.seedCoreService,
                    blocksToGenerateInOneStep,
                    NETWORK_LOCAL,
                    // eslint-disable-next-line no-loop-func
                    (blocks) => {
                      blocksGenerated += blocks;

                      observer.next(`${blocksGenerated} blocks generated`);
                    },
                  );
                } while (!isDip8Activated);

                observer.next(`DIP8 has been activated at height ${blockchainInfo.bip9_softforks.dip0008.since}`);

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
              title: 'Wait for quorums to be enabled',
              task: () => enableCoreQuorumsTask(),
            },
            {
              title: 'Setting initial core chain locked height',
              task: async (_, task) => {
                const rpcClient = ctx.seedCoreService.getRpcClient();
                const { result: initialCoreChainLockedHeight } = await rpcClient.getBlockCount();

                ctx.initialCoreChainLockedHeight = initialCoreChainLockedHeight;

                // eslint-disable-next-line no-param-reassign
                task.output = `Initial chain locked core height is set to: ${ctx.initialCoreChainLockedHeight}`;
              },
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

module.exports = configureCoreTaskFactory;
