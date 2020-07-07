const { Listr } = require('listr2');

const { Observable } = require('rxjs');

const dpnsDocumentSchema = require('@dashevo/dpns-contract/src/schema/dpns-documents.json');

const wait = require('../../../util/wait');

const PRESETS = require('../../../presets');

/**
 *
 * @param {DockerCompose} dockerCompose
 * @param {startNodeTask} startNodeTask
 * @param {createClientWithFundedWallet} createClientWithFundedWallet
 * @param {waitForBlocksWithSDK} waitForBlocksWithSDK
 * @param {generateBlocksWithSDK} generateBlocksWithSDK
 * @return {initTask}
 */
function initTaskFactory(
  dockerCompose,
  startNodeTask,
  createClientWithFundedWallet,
  waitForBlocksWithSDK,
  generateBlocksWithSDK,
) {
  /**
   * @typedef {initTask}
   * @param {string} preset
   * @return {Listr}
   */
  function initTask(
    preset,
  ) {
    return new Listr([
      {
        title: `Start masternode with ${preset} preset`,
        task: async (ctx) => startNodeTask(
          preset,
          {
            externalIp: ctx.externalIp,
            coreP2pPort: ctx.coreP2pPort,
            operatorPrivateKey: ctx.operator.privateKey,
            driveImageBuildPath: ctx.driveImageBuildPath,
            dapiImageBuildPath: ctx.dapiImageBuildPath,
          },
        ),
      },
      {
        title: 'Initialize SDK',
        task: async (ctx, task) => {
          // wait 5 seconds to ensure everything was initialized
          await wait(5000);

          ctx.client = await createClientWithFundedWallet(
            preset,
            ctx.network,
            ctx.fundingPrivateKeyString,
          );

          // eslint-disable-next-line no-param-reassign
          task.output = `HD private key: ${ctx.client.wallet.exportWallet('HDPrivateKey')}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Register DPNS identity',
        task: async (ctx, task) => {
          ctx.identity = await ctx.client.platform.identities.register(5);

          // eslint-disable-next-line no-param-reassign
          task.output = `DPNS identity: ${ctx.identity.getId()}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Register DPNS contract',
        task: async (ctx, task) => {
          ctx.dataContract = await ctx.client.platform.contracts.create(
            dpnsDocumentSchema, ctx.identity,
          );

          await ctx.client.platform.contracts.broadcast(
            ctx.dataContract,
            ctx.identity,
          );

          // eslint-disable-next-line no-param-reassign
          task.output = `DPNS contract ID: ${ctx.dataContract.getId()}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Register top level domain "dash"',
        task: async (ctx) => {
          ctx.client.apps.dpns = {
            contractId: ctx.dataContract.getId(),
          };

          await ctx.client.platform.names.register('dash', ctx.identity);
        },
      },
      {
        title: 'Mine 100 blocks to confirm',
        enabled: () => preset === PRESETS.LOCAL,
        task: async (ctx) => (
          new Observable(async (observer) => {
            await generateBlocksWithSDK(
              ctx.client.getDAPIClient(),
              ctx.network,
              100,
              (blocks) => {
                observer.next(`${blocks} ${blocks > 1 ? 'blocks' : 'block'} mined`);
              },
            );

            observer.complete();
          })
        ),
      },
      {
        title: 'Wait 100 blocks to be mined',
        enabled: () => preset === PRESETS.EVONET,
        task: async (ctx) => (
          new Observable(async (observer) => {
            await waitForBlocksWithSDK(
              ctx.client.getDAPIClient(),
              100,
              (blocks) => {
                observer.next(`${blocks} ${blocks > 1 ? 'blocks' : 'block'} mined`);
              },
            );

            observer.complete();
          })
        ),
      },
      {
        title: 'Disconnect SDK',
        task: async (ctx) => ctx.client.disconnect(),
      },
      {
        title: 'Stop node',
        task: async () => dockerCompose.stop(preset),
      },
    ]);
  }

  return initTask;
}

module.exports = initTaskFactory;
