import { Listr } from 'listr2';
import { Observable } from 'rxjs';
import DashCoreLib from '@dashevo/dashcore-lib';
import waitForNodesToHaveTheSameHeight from '../../../../core/waitForNodesToHaveTheSameHeight.js';
import waitForNodesToHaveTheSameSporks from '../../../../core/waitForNodesToHaveTheSameSporks.js';

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
 * @param {enableCoreQuorumsTask} enableCoreQuorumsTask
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
  enableCoreQuorumsTask,
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
              title: 'Activating DIP3',
              task: () => new Observable(async (observer) => {
                const dip3ActivationHeight = 1000;
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
                const amount = HPMN_COLLATERAL_AMOUNT * configGroup.length;
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

                  isDip8Activated = blockchainInfo.softforks.dip0008.active;

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

                observer.next(`DIP8 has been activated at height ${blockchainInfo.softforks.dip0008.height}`);

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
              title: 'Activating V20 fork',
              task: () => new Observable(async (observer) => {
                let isV20Activated = false;
                let blockchainInfo;

                let blocksGenerated = 0;

                const blocksToGenerateInOneStep = 10;

                do {
                  ({
                    result: blockchainInfo,
                  } = await ctx.seedCoreService.getRpcClient().getBlockchainInfo());

                  isV20Activated = blockchainInfo.softforks && blockchainInfo.softforks.v20
                    && blockchainInfo.softforks.v20.active;
                  if (isV20Activated) {
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
                } while (!isV20Activated);

                observer.next(`V20 fork has been activated at height ${blockchainInfo.softforks.v20.height}`);

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
              title: 'Enable EHF spork',
              task: async () => new Observable(async (observer) => {
                const seedRpcClient = ctx.seedCoreService.getRpcClient();
                const {
                  result: initialCoreChainLockedHeight,
                } = await seedRpcClient.getBlockCount();

                await activateCoreSpork(
                  seedRpcClient,
                  'SPORK_24_TEST_EHF',
                  initialCoreChainLockedHeight,
                );

                let isEhfActivated = false;
                let blockchainInfo;

                let blocksGenerated = 0;

                const blocksToGenerateInOneStep = 48;

                do {
                  ({
                    result: blockchainInfo,
                  } = await ctx.seedCoreService.getRpcClient().getBlockchainInfo());

                  isEhfActivated = blockchainInfo.softforks && blockchainInfo.softforks.mn_rr
                    && blockchainInfo.softforks.mn_rr.active;
                  if (isEhfActivated) {
                    break;
                  }

                  await ctx.bumpMockTime(blocksToGenerateInOneStep);

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
                } while (!isEhfActivated);

                observer.next(`EHF has been activated at height ${blockchainInfo.softforks.mn_rr.height}`);

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
