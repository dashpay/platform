const Listr = require('listr');
const { Observable } = require('rxjs');

const { PrivateKey } = require('@dashevo/dashcore-lib');

const BaseCommand = require('../oclif/command/BaseCommand');
const UpdateRendererWithOutput = require('../oclif/renderer/UpdateRendererWithOutput');
const MutedError = require('../oclif/errors/MutedError');

const PRESETS = require('../presets');

const MASTERNODE_DASH_AMOUNT = 1000;

class RegisterCommand extends BaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
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
   * @return {Promise<void>}
   */
  async runWithDependencies(
    {
      preset, port, 'funding-private-key': fundingPrivateKeyString, 'external-ip': externalIp,
    },
    flags,
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
    const network = 'testnet';

    const fundingPrivateKey = new PrivateKey(
      fundingPrivateKeyString,
      network,
    );

    const fundingAddress = fundingPrivateKey.toAddress(network).toString();

    const tasks = new Listr([
      {
        title: `Register masternode using ${preset} preset`,
        task: () => (
          new Listr([
            {
              title: 'Start Core',
              task: async (ctx) => {
                ctx.coreService = await startCore(preset, { wallet: true });
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
                if (balance <= MASTERNODE_DASH_AMOUNT) {
                  throw new Error('You need to have more than 1000 Dash on your funding address');
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
              title: 'Send 1000 dash from funding address to collateral address',
              task: async (ctx, task) => {
                ctx.collateralTxId = await sendToAddress(
                  ctx.coreService,
                  ctx.fundingPrivateKeyString,
                  ctx.fundingAddress,
                  ctx.collateral.address,
                  1000,
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
                  ctx.port,
                  ctx.owner.address,
                  ctx.operator.publicKey,
                  ctx.fundingAddress,
                );

                // eslint-disable-next-line no-param-reassign
                task.output = `ProRegTx transaction ID: ${proRegTx}\nDon't forget to add bls private key to your configuration`;
              },
            },
          ])
        ),
      },
    ],
    { collapse: false, renderer: UpdateRendererWithOutput });

    try {
      await tasks.run({
        fundingAddress,
        fundingPrivateKeyString,
        externalIp,
        port,
      });
    } catch (e) {
      // we already output errors through listr
      throw new MutedError(e);
    }
  }
}

RegisterCommand.description = `Register masternode
...
Register masternode using predefined presets
`;

RegisterCommand.args = [{
  name: 'preset',
  required: true,
  description: 'preset to use',
  options: Object.values(PRESETS),
}, {
  name: 'funding-private-key',
  required: true,
  description: 'private key with more than 1000 dash for funding collateral',
}, {
  name: 'external-ip',
  required: true,
  description: 'masternode external IP',
}, {
  name: 'port',
  required: true,
  description: 'masternode P2P port',
}];

module.exports = RegisterCommand;
