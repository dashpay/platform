const Listr = require('listr');

const { Observable } = require('rxjs');

const PRESETS = require('../../presets');

const UpdateRendererWithOutput = require('../../oclif/renderer/UpdateRendererWithOutput');

const masternodeDashAmount = require('../../core/masternodeDashAmount');

/**
 *
 * @param {startCore} startCore
 * @param {createNewAddress} createNewAddress
 * @param {generateToAddress} generateToAddress
 * @param {generateBlocks} generateBlocks
 * @param {waitForCoreSync} waitForCoreSync
 * @param {importPrivateKey} importPrivateKey
 * @param {getAddressBalance} getAddressBalance
 * @param {generateBlsKeys} generateBlsKeys
 * @param {sendToAddress} sendToAddress
 * @param {waitForConfirmations} waitForConfirmations
 * @param {registerMasternode} registerMasternode
 * @return {registerMasternodeTask}
 */
function registerMasternodeTaskFactory(
  startCore,
  createNewAddress,
  generateToAddress,
  generateBlocks,
  waitForCoreSync,
  importPrivateKey,
  getAddressBalance,
  generateBlsKeys,
  sendToAddress,
  waitForConfirmations,
  registerMasternode,
) {
  /**
   * @typedef {registerMasternodeTask}
   * @param {string} preset
   * @return {Listr}
   */
  function registerMasternodeTask(preset) {
    return new Listr([
      {
        title: 'Start Core',
        task: async (ctx) => {
          ctx.coreService = await startCore(preset, { wallet: true, addressindex: true });
        },
      },
      {
        title: 'Import funding private key',
        task: async (ctx) => importPrivateKey(ctx.coreService, ctx.fundingPrivateKeyString),
      },
      {
        title: 'Sync Core with network',
        enabled: () => preset !== PRESETS.LOCAL,
        task: async (ctx) => waitForCoreSync(ctx.coreService),
      },
      {
        title: 'Check funding address balance',
        task: async (ctx) => {
          const balance = await getAddressBalance(ctx.coreService, ctx.fundingAddress);
          if (balance <= masternodeDashAmount) {
            throw new Error(`You need to have more than ${masternodeDashAmount} Dash on your funding address`);
          }
        },
      },
      {
        title: 'Generate a masternode operator key',
        task: async (ctx, task) => {
          ctx.operator = await generateBlsKeys();

          // eslint-disable-next-line no-param-reassign
          task.output = `Public key: ${ctx.operator.publicKey}\nPrivate key: ${ctx.operator.privateKey}`;
        },
      },
      {
        title: 'Create a new collateral address',
        task: async (ctx, task) => {
          ctx.collateral = await createNewAddress(ctx.coreService);

          // eslint-disable-next-line no-param-reassign
          task.output = `Address: ${ctx.collateral.address}\nPrivate key: ${ctx.collateral.privateKey}`;
        },
      },
      {
        title: 'Create a new owner addresses',
        task: async (ctx, task) => {
          ctx.owner = await createNewAddress(ctx.coreService);

          // eslint-disable-next-line no-param-reassign
          task.output = `Address: ${ctx.owner.address}\nPrivate key: ${ctx.owner.privateKey}`;
        },
      },
      {
        title: `Send ${masternodeDashAmount} dash from funding address to collateral address`,
        task: async (ctx, task) => {
          ctx.collateralTxId = await sendToAddress(
            ctx.coreService,
            ctx.fundingPrivateKeyString,
            ctx.fundingAddress,
            ctx.collateral.address,
            masternodeDashAmount,
          );

          // eslint-disable-next-line no-param-reassign
          task.output = `Collateral transaction ID: ${ctx.collateralTxId}`;
        },
      },
      {
        title: 'Wait for 15 confirmations',
        enabled: () => preset !== PRESETS.LOCAL,
        task: async (ctx) => (
          new Observable(async (observer) => {
            await waitForConfirmations(
              ctx.coreService,
              ctx.collateralTxId,
              15,
              (confirmations) => {
                observer.next(`${confirmations} ${confirmations > 1 ? 'confirmations' : 'confirmation'}`);
              },
            );

            observer.complete();
          })
        ),
      },
      {
        title: 'Mine 15 blocks to confirm',
        enabled: () => preset === PRESETS.LOCAL,
        task: async (ctx) => (
          new Observable(async (observer) => {
            await generateBlocks(
              ctx.coreService,
              15,
              (blocks) => {
                observer.next(`${blocks} ${blocks > 1 ? 'blocks' : 'block'} mined`);
              },
            );

            observer.complete();
          })
        ),
      },
      {
        title: 'Reach 1000 blocks to enable DML',
        enabled: () => preset === PRESETS.LOCAL,
        // eslint-disable-next-line consistent-return
        task: async (ctx) => {
          const { result: height } = await ctx.coreService.getRpcClient().getBlockCount();

          if (height < 1000) {
            return new Observable(async (observer) => {
              await generateBlocks(
                ctx.coreService,
                1000 - height,
                (blocks) => {
                  const remaining = 1000 - height - blocks;
                  observer.next(`${remaining} ${remaining > 1 ? 'blocks' : 'block'} remaining`);
                },
              );

              observer.complete();
            });
          }
        },
      },
      {
        title: 'Broadcast masternode registration transaction',
        task: async (ctx, task) => {
          const proRegTx = await registerMasternode(
            ctx.coreService,
            ctx.collateralTxId,
            ctx.externalIp,
            ctx.coreP2pPort,
            ctx.owner.address,
            ctx.operator.publicKey,
            ctx.fundingAddress,
          );

          // eslint-disable-next-line no-param-reassign
          task.output = `ProRegTx transaction ID: ${proRegTx}\nDon't forget to add bls private key to your configuration`;
        },
      },
      {
        title: 'Wait for 1 confirmation',
        enabled: () => preset !== PRESETS.LOCAL,
        task: async (ctx) => (
          new Observable(async (observer) => {
            await waitForConfirmations(
              ctx.coreService,
              ctx.collateralTxId,
              1,
              (confirmations) => {
                observer.next(`${confirmations} ${confirmations > 1 ? 'confirmations' : 'confirmation'}`);
              },
            );

            observer.complete();
          })
        ),
      },
      {
        title: 'Mine 1 block to confirm',
        enabled: () => preset === PRESETS.LOCAL,
        task: async (ctx) => (
          new Observable(async (observer) => {
            await generateBlocks(
              ctx.coreService,
              1,
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
    ],
    {
      collapse: false,
      renderer: UpdateRendererWithOutput,
    });
  }

  return registerMasternodeTask;
}

module.exports = registerMasternodeTaskFactory;
