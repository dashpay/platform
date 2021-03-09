const { Listr } = require('listr2');
const {
  PrivateKey,
} = require('@dashevo/dashcore-lib');
const isSeedNode = require('../../../../util/isSeedNode');

/**
 * @param {resolveDockerHostIp} resolveDockerHostIp
 * @param {renderServiceTemplates} renderServiceTemplates
 * @param {writeServiceConfigs} writeServiceConfigs
 * @param {startCore} startCore
 * @param {generateBlocks} generateBlocks
 * @param {waitForCoreSync} waitForCoreSync
 * @param {activateCoreSpork} activateCoreSpork
 * @param {waitForCoreQuorum} waitForCoreQuorum
 * @param {generateToAddressTask} generateToAddressTask
 * @param {registerMasternodeTask} registerMasternodeTask
 * @param {generateBlsKeys} generateBlsKeys
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
  waitForCoreQuorum,
  generateToAddressTask,
  registerMasternodeTask,
  generateBlsKeys,
) {
  /**
   * @typedef {configureCoreTask}
   * @param {Config[]} configGroup
   * @return {Listr}
   */
  function configureCoreTask(configGroup) {
    const amount = 1100;

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
            // seeds
            config.set(
              'core.p2p.seeds',
              p2pSeeds.filter((seed, index) => index !== i),
            );

            // sporks
            config.set(
              'core.spork.address',
              sporkAddress,
            );

            if (isSeedNode(config)) {
              config.set(
                'core.spork.privateKey',
                sporkPrivKey.toWIF(),
              );
            }

            // Write configs
            const configFiles = renderServiceTemplates(config);
            writeServiceConfigs(config.getName(), configFiles);
          });

          return new Listr([
            {
              title: 'Starting Core nodes',
              task: async () => {
                const coreServices = [];

                let isGenesisBlockGenerated = false;

                for (const config of configGroup) {
                  const coreService = await startCore(config, { wallet: true, addressIndex: true });
                  coreServices.push(coreService);

                  // need to generate 1 block to connect nodes to each other
                  if (!isGenesisBlockGenerated) {
                    await generateBlocks(
                      coreService,
                      1,
                      config.get('network'),
                    );

                    isGenesisBlockGenerated = true;
                  }
                }

                ctx.coreServices = coreServices;
              },
            },
            {
              title: 'Register masternodes',
              task: () => {
                const masternodeConfigs = configGroup.filter((config) => config.get('core.masternode.enable'));

                const subTasks = masternodeConfigs.map((config, i) => ({
                  title: `Register ${config.getName()} masternode`,
                  skip: () => {
                    if (config.get('core.masternode.operator.privateKey')) {
                      return `Masternode operator private key ('core.masternode.operator.privateKey') is already set in ${config.getName()} config`;
                    }

                    return false;
                  },
                  task: () => new Listr([
                    {
                      task: () => {
                        ctx.coreService = ctx.coreServices[i];
                      },
                    },
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
                      title: 'Await for Core to sync',
                      enabled: () => i > 0,
                      task: () => waitForCoreSync(ctx.coreService),
                    },
                    {
                      title: `Generate ${amount} dash to local wallet`,
                      task: () => generateToAddressTask(config, amount),
                    },
                    {
                      task: () => registerMasternodeTask(config),
                    },
                    {
                      // hidden task to clear values
                      task: async () => {
                        ctx.address = null;
                        ctx.privateKey = null;
                        ctx.coreService = null;
                      },
                    },
                  ]),
                }));

                // eslint-disable-next-line consistent-return
                return new Listr(subTasks);
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
