const { Listr } = require('listr2');

const Dash = require('dash');

const crypto = require('crypto');

const fundWallet = require('@dashevo/wallet-lib/src/utils/fundWallet');

const dpnsDocumentSchema = require('@dashevo/dpns-contract/schema/dpns-contract-documents.json');
const dashpayDocumentSchema = require('@dashevo/dashpay-contract/schema/dashpay.schema.json');

const NETWORKS = require('../../../networks');

/**
 *
 * @return {initTask}
 */
function initTaskFactory(
  createTenderdashRpcClient,
) {
  /**
   * @typedef {initTask}
   * @param {Config} config
   * @return {Listr}
   */
  function initTask(
    config,
  ) {
    const dpnsOwnerId = config.get('platform.dpns.ownerId');

    if (dpnsOwnerId !== null) {
      throw new Error(`DPNS owner ID ('platform.dpns.ownerId') is already set in ${config.getName()} config`);
    }

    const dpnsContractId = config.get('platform.dpns.contract.id');

    if (dpnsContractId !== null) {
      throw new Error(`DPNS owner ID ('platform.dpns.contract.id') is already set in ${config.getName()} config`);
    }

    return new Listr([
      {
        title: 'Initialize SDK',
        task: async (ctx, task) => {
          const clientOpts = {
            network: config.get('network'),
          };

          if (ctx.seed) {
            clientOpts.seeds = [ctx.seed];
          }

          const faucetClient = new Dash.Client({
            ...clientOpts,
            wallet: {
              privateKey: ctx.fundingPrivateKeyString,
            },
          });

          ctx.client = new Dash.Client({
            ...clientOpts,
            wallet: {
              mnemonic: null,
            },
            passFakeAssetLockProofForTests: true,
          });

          const amount = 40000;

          await fundWallet(faucetClient.wallet, ctx.client.wallet, amount);

          await faucetClient.disconnect();

          // eslint-disable-next-line no-param-reassign
          task.output = `HD private key: ${ctx.client.wallet.exportWallet('HDPrivateKey')}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Register DPNS identity',
        task: async (ctx, task) => {
          ctx.identity = await ctx.client.platform.identities.register(5);

          config.set('platform.dpns.ownerId', ctx.identity.getId().toString());

          // eslint-disable-next-line no-param-reassign
          task.output = `DPNS identity: ${ctx.identity.getId().toString()}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Register DPNS contract',
        task: async (ctx, task) => {
          ctx.dataContract = await ctx.client.platform.contracts.create(
            dpnsDocumentSchema, ctx.identity,
          );

          ctx.dataContractStateTransition = await ctx.client.platform.contracts.broadcast(
            ctx.dataContract,
            ctx.identity,
          );

          config.set('platform.dpns.contract.id', ctx.dataContract.getId().toString());

          // eslint-disable-next-line no-param-reassign
          task.output = `DPNS contract ID: ${ctx.dataContract.getId().toString()}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Obtain DPNS contract commit block height',
        task: async (ctx, task) => {
          const stateTransitionHash = crypto.createHash('sha256')
            .update(ctx.dataContractStateTransition.toBuffer())
            .digest();

          if (ctx.seed || config.get('network') !== NETWORKS.LOCAL) {
            task.skip('Can\'t obtain DPNS contract commit block height from remote node.'
              + `Please, get block height manually using state transition hash "0x${stateTransitionHash.toString('hex')}"`
              + 'and set it to "platform.dpns.contract.id" config option');

            return;
          }

          const tenderdashRpcClient = createTenderdashRpcClient();

          const params = { hash: stateTransitionHash.toString('base64') };

          const response = await tenderdashRpcClient.request('tx', params);

          if (response.error) {
            throw new Error(`Tendermint error: ${response.error.message}: ${response.error.data}`);
          }

          const { result: { height: contractBlockHeight } } = response;

          config.set('platform.dpns.contract.blockHeight', contractBlockHeight);

          // eslint-disable-next-line no-param-reassign
          task.output = `DPNS contract block height: ${contractBlockHeight}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Register top level domain "dash"',
        task: async (ctx) => {
          ctx.client.getApps().set('dpns', {
            contractId: ctx.dataContract.getId(),
            contract: ctx.dataContract,
          });

          await ctx.client.platform.names.register('dash', {
            dashAliasIdentityId: ctx.identity.getId(),
          }, ctx.identity);
        },
      },
      {
        title: 'Register identity for Dashpay',
        task: async (ctx, task) => {
          ctx.identity = await ctx.client.platform.identities.register(5);

          // eslint-disable-next-line no-param-reassign
          task.output = `Dashpay's owner identity: ${ctx.identity.getId()}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Register Dashpay Contract',
        task: async (ctx, task) => {
          ctx.dataContract = await ctx.client.platform.contracts.create(
            dashpayDocumentSchema, ctx.identity,
          );

          await ctx.client.platform.contracts.broadcast(
            ctx.dataContract,
            ctx.identity,
          );

          // eslint-disable-next-line no-param-reassign
          task.output = `Dashpay contract ID: ${ctx.dataContract.getId()}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Disconnect SDK',
        task: async (ctx) => ctx.client.disconnect(),
      },
    ]);
  }

  return initTask;
}

module.exports = initTaskFactory;
