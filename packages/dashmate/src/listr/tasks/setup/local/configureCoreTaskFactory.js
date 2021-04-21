const { Listr } = require('listr2');
const { Observable } = require('rxjs');

const {
  PrivateKey,
} = require('@dashevo/dashcore-lib');

const waitForNodesToHaveTheSameSporks = require('../../../../core/waitForNodesToHaveTheSameSporks');

const { NETWORK_LOCAL, MASTERNODE_DASH_AMOUNT } = require('../../../../constants');

/**
 * @param {resolveDockerHostIp} resolveDockerHostIp
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
 * @return {configureCoreTask}
 */
function configureCoreTaskFactory(
  resolveDockerHostIp,
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
) {
  /**
   * @typedef {configureCoreTask}
   * @param {Config[]} configGroup
   * @return {Listr}
   */
  function configureCoreTask(configGroup) {
    return new Listr([
      {
        task: async (ctx) => {
          if (!ctx.hostDockerInternalIp) {
            ctx.hostDockerInternalIp = await resolveDockerHostIp();
          }

          const network = configGroup[0].get('network');
          const sporkPrivKey = new PrivateKey(undefined, network);
          const sporkAddress = sporkPrivKey.toAddress(network).toString();

          const p2pSeeds = configGroup.map((config) => ({
            host: ctx.hostDockerInternalIp,
            port: config.get('core.p2p.port'),
          }));

          configGroup.forEach((config, i) => {
            // Set seeds
            config.set(
              'core.p2p.seeds',
              p2pSeeds.filter((seed, index) => index !== i),
            );

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
              title: 'Starting a wallet',
              task: async () => {
                const config = configGroup[0];

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
                  configGroup[0],
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
                    activateCoreSpork(ctx.coreService.getRpcClient(), spork))),
                );

                await waitForNodesToHaveTheSameSporks(ctx.coreServices);
              },
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
                  } = await ctx.coreService.getRpcClient().getBlockchainInfo());

                  isDip8Activated = blockchainInfo.bip9_softforks.dip0008.status === 'active';

                  if (isDip8Activated) {
                    break;
                  }

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
                } while (!isDip8Activated);

                observer.next(`DIP8 has been activated at height ${blockchainInfo.bip9_softforks.dip0008.since}`);

                observer.complete();

                return this;
              }),
            },
            {
              title: 'Stopping wallet',
              task: async () => {
                await ctx.coreService.stop();
              },
            },
            {
              title: 'Starting masternodes',
              task: async () => {
                const coreServices = [];

                for (const config of configGroup) {
                  const coreService = await startCore(config);
                  coreServices.push(coreService);
                }

                ctx.coreServices = coreServices;
              },
            },
            {
              title: 'Wait for core quorums to be enabled',
              task: () => enableCoreQuorumsTask(),
            },
            {
              title: 'Setting initial core chain locked height',
              task: async (_, task) => {
                const rpcClient = ctx.coreServices[0].getRpcClient();
                const { result: initialCoreChainLockedHeight } = await rpcClient.getBlockCount();

                ctx.initialCoreChainLockedHeight = initialCoreChainLockedHeight;

                // eslint-disable-next-line no-param-reassign
                task.output = `Initial chain locked core height is set to: ${ctx.initialCoreChainLockedHeight}`;
              },
            },
            {
              title: 'Stopping masternodes',
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
