const { Listr } = require('listr2');

const { Observable } = require('rxjs');

const PRESETS = require('../../../presets');

/**
 *
 * @param {startCore} startCore
 * @param {createNewAddress} createNewAddress
 * @param {generateToAddress} generateToAddress
 * @param {generateBlocks} generateBlocks
 * @param {waitForCoreSync} waitForCoreSync
 * @param {waitForBlocks} waitForBlocks
 * @return {generateToAddressTask}
 */
function generateToAddressTaskFactory(
  startCore,
  createNewAddress,
  generateToAddress,
  generateBlocks,
  waitForCoreSync,
  waitForBlocks,
) {
  /**
   * @typedef {generateToAddressTask}
   * @param {string} preset
   * @param {number} amount
   * @return {Listr}
   */
  function generateToAddressTask(preset, amount) {
    return new Listr([
      {
        title: 'Start Core',
        task: async (ctx) => {
          ctx.coreService = await startCore(preset, { wallet: true });
        },
      },
      {
        title: 'Sync Core with network',
        enabled: () => preset !== PRESETS.LOCAL,
        task: async (ctx) => waitForCoreSync(ctx.coreService),
      },
      {
        title: 'Create a new address',
        skip: (ctx) => {
          if (ctx.fundingAddress) {
            return `Use specified address ${ctx.fundingAddress}`;
          }

          return false;
        },
        task: async (ctx, task) => {
          ({
            address: ctx.fundingAddress,
            privateKey: ctx.fundingPrivateKeyString,
          } = await createNewAddress(ctx.coreService));

          // eslint-disable-next-line no-param-reassign
          task.output = `Address: ${ctx.fundingAddress}\nPrivate key: ${ctx.fundingPrivateKeyString}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: `Generate ≈${amount} dash to address`,
        task: (ctx, task) => (
          new Observable(async (observer) => {
            await generateToAddress(
              ctx.coreService,
              amount,
              ctx.fundingAddress,
              (balance) => {
                ctx.balance = balance;
                observer.next(`${balance} dash generated`);
              },
            );

            // eslint-disable-next-line no-param-reassign
            task.output = `Generated ${ctx.balance} dash`;

            observer.complete();
          })
        ),
        options: { persistentOutput: true },
      },
      {
        title: 'Mine 100 blocks to confirm',
        enabled: () => preset === PRESETS.LOCAL,
        task: async (ctx) => (
          new Observable(async (observer) => {
            await generateBlocks(
              ctx.coreService,
              100,
              ctx.network,
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
            await waitForBlocks(
              ctx.coreService,
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
        title: 'Stop Core',
        task: async (ctx) => ctx.coreService.stop(),
      },
    ]);
  }

  return generateToAddressTask;
}

module.exports = generateToAddressTaskFactory;
