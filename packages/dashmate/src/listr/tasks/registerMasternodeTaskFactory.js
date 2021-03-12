const { Listr } = require('listr2');

const { Observable } = require('rxjs');

const {
  NETWORK_LOCAL,
  MASTERNODE_DASH_AMOUNT,
} = require('../../constants');

/**
 *
 * @param {startCore} startCore
 * @param {createNewAddress} createNewAddress
 * @param {generateToAddress} generateToAddress
 * @param {generateBlocks} generateBlocks
 * @param {waitForCoreSync} waitForCoreSync
 * @param {importPrivateKey} importPrivateKey
 * @param {getAddressBalance} getAddressBalance
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
  sendToAddress,
  waitForConfirmations,
  registerMasternode,
) {
  /**
   * @typedef {registerMasternodeTask}
   * @param {Config} config
   * @return {Listr}
   */
  function registerMasternodeTask(config) {
    return new Listr([
      {
        title: 'Start Core',
        enabled: (ctx) => {
          ctx.coreServicePassed = Boolean(ctx.coreService);

          return !ctx.coreServicePassed;
        },
        task: async (ctx) => {
          ctx.coreServicePassed = false;
          ctx.coreService = await startCore(config, { wallet: true, addressIndex: true });
        },
      },
      {
        title: 'Import funding private key',
        task: async (ctx, task) => {
          await importPrivateKey(ctx.coreService, ctx.fundingPrivateKeyString);

          // eslint-disable-next-line no-param-reassign
          task.output = `${ctx.fundingPrivateKeyString} imported.`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Sync Core with network',
        enabled: () => config.get('network') !== NETWORK_LOCAL,
        task: async (ctx) => (
          new Observable(async (observer) => {
            await waitForCoreSync(
              ctx.coreService.getRpcClient(),
              (verificationProgress) => {
                observer.next(`${(verificationProgress * 100).toFixed(2)}% complete`);
              },
            );

            observer.complete();

            return this;
          })
        ),
      },
      {
        title: 'Check funding address balance',
        task: async (ctx, task) => {
          // eslint-disable-next-line no-param-reassign
          task.title = `Check funding address ${ctx.fundingAddress} balance`;

          const balance = await getAddressBalance(ctx.coreService, ctx.fundingAddress);

          if (balance <= MASTERNODE_DASH_AMOUNT) {
            throw new Error(`You need to have more than ${MASTERNODE_DASH_AMOUNT} Dash on your funding address`);
          }
        },
      },
      {
        title: 'Create a new collateral address',
        task: async (ctx, task) => {
          ctx.collateral = await createNewAddress(ctx.coreService);

          // eslint-disable-next-line no-param-reassign
          task.output = `Address: ${ctx.collateral.address}\nPrivate key: ${ctx.collateral.privateKey}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Create a new owner addresses',
        task: async (ctx, task) => {
          ctx.owner = await createNewAddress(ctx.coreService);

          // eslint-disable-next-line no-param-reassign
          task.output = `Address: ${ctx.owner.address}\nPrivate key: ${ctx.owner.privateKey}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: `Send ${MASTERNODE_DASH_AMOUNT} dash from funding address to collateral address`,
        task: async (ctx, task) => {
          ctx.collateralTxId = await sendToAddress(
            ctx.coreService,
            ctx.fundingPrivateKeyString,
            ctx.fundingAddress,
            ctx.collateral.address,
            MASTERNODE_DASH_AMOUNT,
          );

          // eslint-disable-next-line no-param-reassign
          task.output = `Collateral transaction ID: ${ctx.collateralTxId}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Wait for 1 confirmation',
        enabled: () => config.get('network') !== NETWORK_LOCAL,
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

            return this;
          })
        ),
      },
      {
        title: 'Mine 15 blocks to confirm',
        enabled: () => config.get('network') === NETWORK_LOCAL,
        task: async (ctx) => (
          new Observable(async (observer) => {
            await generateBlocks(
              ctx.coreService,
              15,
              config.get('network'),
              (blocks) => {
                observer.next(`${blocks} ${blocks > 1 ? 'blocks' : 'block'} mined`);
              },
            );

            observer.complete();

            return this;
          })
        ),
      },
      {
        title: 'Reach 1000 blocks to enable DML',
        enabled: () => config.get('network') === NETWORK_LOCAL,
        // eslint-disable-next-line consistent-return
        task: async (ctx, task) => {
          const { result: blockCount } = await ctx.coreService.getRpcClient().getBlockCount();

          if (blockCount >= 1000) {
            // eslint-disable-next-line no-param-reassign
            task.skip = true;

            return;
          }

          // eslint-disable-next-line consistent-return
          return new Observable(async (observer) => {
            await generateBlocks(
              ctx.coreService,
              1000 - blockCount,
              config.get('network'),
              (blocks) => {
                const remaining = 1000 - blockCount - blocks;
                observer.next(`${remaining} ${remaining > 1 ? 'blocks' : 'block'} remaining`);
              },
            );

            observer.complete();

            return this;
          });
        },
      },
      {
        title: 'Broadcast masternode registration transaction',
        task: async (ctx, task) => {
          const proRegTx = await registerMasternode(
            ctx.coreService,
            ctx.collateralTxId,
            ctx.owner.address,
            ctx.operator.publicKey,
            ctx.fundingAddress,
            config,
          );

          // eslint-disable-next-line no-param-reassign
          task.output = `ProRegTx transaction ID: ${proRegTx}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Wait for 1 confirmation',
        enabled: () => config.get('network') !== NETWORK_LOCAL,
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

            return this;
          })
        ),
      },
      {
        title: 'Mine 1 block to confirm',
        enabled: () => config.get('network') === NETWORK_LOCAL,
        task: async (ctx) => (
          new Observable(async (observer) => {
            await generateBlocks(
              ctx.coreService,
              1,
              config.get('network'),
              (blocks) => {
                observer.next(`${blocks} ${blocks > 1 ? 'blocks' : 'block'} mined`);
              },
            );

            observer.complete();

            return this;
          })
        ),
      },
      {
        title: 'Stop Core',
        enabled: (ctx) => !ctx.coreServicePassed,
        task: async (ctx) => ctx.coreService.stop(),
      },
    ]);
  }

  return registerMasternodeTask;
}

module.exports = registerMasternodeTaskFactory;
