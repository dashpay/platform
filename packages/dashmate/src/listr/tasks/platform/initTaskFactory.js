const { Listr } = require('listr2');

const Dash = require('dash');

const crypto = require('crypto');

const fundWallet = require('@dashevo/wallet-lib/src/utils/fundWallet');

const dpnsDocumentSchema = require('@dashevo/dpns-contract/schema/dpns-contract-documents.json');
const dashpayDocumentSchema = require('@dashevo/dashpay-contract/schema/dashpay.schema.json');
const featureFlagsDocumentSchema = require('@dashevo/feature-flags-contract/schema/feature-flags-documents.json');

const { NETWORK_LOCAL } = require('../../../constants');

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

          if (ctx.dapiAddress) {
            clientOpts.dapiAddresses = [ctx.dapiAddress];
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

          if (ctx.dapiAddress || config.get('network') !== NETWORK_LOCAL) {
            task.skip('Can\'t obtain DPNS contract commit block height from remote node.'
              + `Please, get block height manually using state transition hash "0x${stateTransitionHash.toString('hex')}"`
              + 'and set it to "platform.dpns.contract.id" config option');

            return;
          }

          const tenderdashRpcClient = createTenderdashRpcClient();

          const params = { hash: stateTransitionHash.toString('base64') };

          const response = await tenderdashRpcClient.request('tx', params);

          if (response.error) {
            throw new Error(`Tenderdash error: ${response.error.message}: ${response.error.data}`);
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

          ctx.dashpayStateTransition = await ctx.client.platform.contracts.broadcast(
            ctx.dataContract,
            ctx.identity,
          );

          config.set('platform.dashpay.contract.id', ctx.dataContract.getId().toString());

          // eslint-disable-next-line no-param-reassign
          task.output = `Dashpay contract ID: ${ctx.dataContract.getId()}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Obtain Dashpay contract commit block height',
        task: async (ctx, task) => {
          const stateTransitionHash = crypto.createHash('sha256')
            .update(ctx.dashpayStateTransition.toBuffer())
            .digest();

          if (ctx.dapiAddress || config.get('network') !== NETWORK_LOCAL) {
            task.skip('Can\'t obtain Dashpay contract commit block height from remote node.'
              + `Please, get block height manually using state transition hash "0x${stateTransitionHash.toString('hex')}"`
              + 'and set it to "platform.dashpay.contract.id" config option');

            return;
          }

          const tenderdashRpcClient = createTenderdashRpcClient();

          const params = { hash: stateTransitionHash.toString('base64') };

          const response = await tenderdashRpcClient.request('tx', params);

          if (response.error) {
            throw new Error(`Tenderdash error: ${response.error.message}: ${response.error.data}`);
          }

          const { result: { height: contractBlockHeight } } = response;

          config.set('platform.dashpay.contract.blockHeight', contractBlockHeight);

          // eslint-disable-next-line no-param-reassign
          task.output = `Dashpay contract block height: ${contractBlockHeight}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Register Feature Fags identity',
        task: async (ctx, task) => {
          ctx.featureFlagsIdentity = await ctx.client.platform.identities.register(5);

          config.set('platform.featureFlags.ownerId', ctx.featureFlagsIdentity.getId().toString());

          // eslint-disable-next-line no-param-reassign
          task.output = `Feature Flags identity: ${ctx.featureFlagsIdentity.getId().toString()}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Register Feature Flags contract',
        task: async (ctx, task) => {
          ctx.featureFlagsDataContract = await ctx.client.platform.contracts.create(
            featureFlagsDocumentSchema, ctx.featureFlagsIdentity,
          );

          ctx.client.getApps().set('featureFlags', {
            contractId: ctx.featureFlagsDataContract.getId(),
            contract: ctx.featureFlagsDataContract,
          });

          ctx.dataContractStateTransition = await ctx.client.platform.contracts.broadcast(
            ctx.featureFlagsDataContract,
            ctx.featureFlagsIdentity,
          );

          config.set('platform.featureFlags.contract.id', ctx.featureFlagsDataContract.getId().toString());

          // eslint-disable-next-line no-param-reassign
          task.output = `Feature Flags contract ID: ${ctx.featureFlagsDataContract.getId().toString()}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Obtain Feature Flags contract commit block height',
        task: async (ctx, task) => {
          const stateTransitionHash = crypto.createHash('sha256')
            .update(ctx.dataContractStateTransition.toBuffer())
            .digest();

          if (ctx.dapiAddress || config.get('network') !== NETWORK_LOCAL) {
            task.skip('Can\'t obtain Feature Flags contract commit block height from remote node.'
              + `Please, get block height manually using state transition hash "0x${stateTransitionHash.toString('hex')}"`
              + 'and set it to "platform.dpns.contract.id" config option');

            return;
          }

          const tenderdashRpcClient = createTenderdashRpcClient();

          const params = { hash: stateTransitionHash.toString('base64') };

          const response = await tenderdashRpcClient.request('tx', params);

          if (response.error) {
            throw new Error(`Tenderdash error: ${response.error.message}: ${response.error.data}`);
          }

          const { result: { height: contractBlockHeight } } = response;

          config.set('platform.featureFlags.contract.blockHeight', contractBlockHeight);

          ctx.featureFlagsContractBlockHeight = contractBlockHeight;

          // eslint-disable-next-line no-param-reassign
          task.output = `Feature Flags contract block height: ${contractBlockHeight}`;
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
