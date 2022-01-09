const { Listr } = require('listr2');

const Dash = require('dash');

const crypto = require('crypto');

const fundWallet = require('@dashevo/wallet-lib/src/utils/fundWallet');

const dpnsSystemIds = require('@dashevo/dpns-contract/lib/systemIds');
const dashpaySystemIds = require('@dashevo/dashpay-contract/lib/systemIds');
const featureFlagsSystemIds = require('@dashevo/feature-flags-contract/lib/systemIds');
const masternodeRewardSharesSystemIds = require('@dashevo/masternode-reward-shares-contract/lib/systemIds');

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
            driveProtocolVersion: 1,
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
      // {
      //   title: 'Top up DPNS identity',
      //   task: async (ctx, task) => {
      //     ctx.identity = await ctx.client.platform.identities.get(
      //       dpnsSystemIds.ownerId,
      //     );
      //
      //     await ctx.client.platform.identities.topUp(ctx.identity.getId(), 5);
      //
      //     config.set('platform.dpns.ownerId', ctx.identity.getId().toString());
      //
      //     // eslint-disable-next-line no-param-reassign
      //     task.output = `DPNS identity: ${ctx.identity.getId()}`;
      //   },
      //   options: { persistentOutput: true },
      // },
      // {
      //   title: 'Setup DPNS contract',
      //   task: async (ctx, task) => {
      //     ctx.dataContract = await ctx.client.platform.contracts.get(
      //       dpnsSystemIds.contractId,
      //     );
      //
      //     config.set('platform.dpns.contract.id', ctx.dataContract.getId().toString());
      //
      //     // eslint-disable-next-line no-param-reassign
      //     task.output = `DPNS contract ID: ${ctx.dataContract.getId().toString()}`;
      //   },
      //   options: { persistentOutput: true },
      // },
      // {
      //   title: 'Obtain DPNS contract commit block height',
      //   task: async (ctx, task) => {
      //     config.set('platform.dpns.contract.blockHeight', 42);
      //
      //     // eslint-disable-next-line no-param-reassign
      //     task.output = `DPNS contract block height: ${42}`;
      //   },
      //   options: { persistentOutput: true },
      // },
      // {
      //   title: 'Top up identity for Dashpay',
      //   task: async (ctx, task) => {
      //     ctx.identity = await ctx.client.platform.identities.get(
      //       dashpaySystemIds.ownerId,
      //     );
      //
      //     await ctx.client.platform.identities.topUp(ctx.identity.getId(), 5);
      //
      //     // eslint-disable-next-line no-param-reassign
      //     task.output = `Dashpay's owner identity: ${ctx.identity.getId()}`;
      //   },
      //   options: { persistentOutput: true },
      // },
      // {
      //   title: 'Setup Dashpay Contract',
      //   task: async (ctx, task) => {
      //     ctx.dataContract = await ctx.client.platform.contracts.get(
      //       dashpaySystemIds.contractId,
      //     );
      //
      //     config.set('platform.dashpay.contract.id', ctx.dataContract.getId().toString());
      //
      //     // eslint-disable-next-line no-param-reassign
      //     task.output = `Dashpay contract ID: ${ctx.dataContract.getId()}`;
      //   },
      //   options: { persistentOutput: true },
      // },
      // {
      //   title: 'Obtain Dashpay contract commit block height',
      //   task: async (ctx, task) => {
      //     config.set('platform.dashpay.contract.blockHeight', 42);
      //
      //     // eslint-disable-next-line no-param-reassign
      //     task.output = `Dashpay contract block height: ${42}`;
      //   },
      //   options: { persistentOutput: true },
      // },
      // {
      //   title: 'Top up Feature Flags identity',
      //   task: async (ctx, task) => {
      //     ctx.featureFlagsIdentity = await ctx.client.platform.identities.get(
      //       featureFlagsSystemIds.ownerId,
      //     );
      //
      //     await ctx.client.platform.identities.topUp(ctx.featureFlagsIdentity.getId(), 5000);
      //
      //     config.set('platform.featureFlags.ownerId', ctx.featureFlagsIdentity.getId().toString());
      //
      //     // eslint-disable-next-line no-param-reassign
      //     task.output = `Feature Flags identity: ${ctx.featureFlagsIdentity.getId().toString()}`;
      //   },
      //   options: { persistentOutput: true },
      // },
      // {
      //   title: 'Setup Feature Flags contract',
      //   task: async (ctx, task) => {
      //     ctx.featureFlagsDataContract = await ctx.client.platform.contracts.get(
      //       featureFlagsSystemIds.contractId,
      //     );
      //
      //     ctx.client.getApps().set('featureFlags', {
      //       contractId: ctx.featureFlagsDataContract.getId(),
      //       contract: ctx.featureFlagsDataContract,
      //     });
      //
      //     config.set('platform.featureFlags.contract.id', ctx.featureFlagsDataContract.getId().toString());
      //
      //     // eslint-disable-next-line no-param-reassign
      //     task.output = `Feature Flags contract ID: ${ctx.featureFlagsDataContract.getId().toString()}`;
      //   },
      //   options: { persistentOutput: true },
      // },
      // {
      //   title: 'Obtain Feature Flags contract commit block height',
      //   task: async (ctx, task) => {
      //     config.set('platform.featureFlags.contract.blockHeight', 42);
      //
      //     ctx.featureFlagsContractBlockHeight = 42;
      //
      //     // eslint-disable-next-line no-param-reassign
      //     task.output = `Feature Flags contract block height: ${42}`;
      //   },
      //   options: { persistentOutput: true },
      // },
      // {
      //   title: 'Top up Masternode Reward Shares identity',
      //   task: async (ctx, task) => {
      //     ctx.masternodeRewardSharesIdentity = await ctx.client.platform.identities.get(
      //       masternodeRewardSharesSystemIds.ownerId,
      //     );
      //
      //     await ctx.client.platform.identities.topUp(
      //       ctx.masternodeRewardSharesIdentity.getId(), 5000,
      //     );
      //
      //     // eslint-disable-next-line no-param-reassign
      //     task.output = `Reward Share identity: ${ctx.masternodeRewardSharesIdentity.getId().toString()}`;
      //   },
      //   options: { persistentOutput: true },
      // },
      // {
      //   title: 'Setup Masternode Reward Share contract',
      //   task: async (ctx, task) => {
      //     ctx.rewardSharingContract = await ctx.client.platform.contracts.get(
      //       masternodeRewardSharesSystemIds.contractId,
      //     );
      //
      //     ctx.client.getApps().set('masternodeRewardShares', {
      //       contractId: ctx.rewardSharingContract.getId(),
      //       contract: ctx.masternodeRewardSharesIdentity,
      //     });
      //
      //     config.set('platform.masternodeRewardShares.contract.id', ctx.rewardSharingContract.getId().toString());
      //
      //     // eslint-disable-next-line no-param-reassign
      //     task.output = `Reward Share contract ID: ${ctx.rewardSharingContract.getId().toString()}`;
      //   },
      //   options: { persistentOutput: true },
      // },
      // {
      //   title: 'Obtain Masternode Reward Share contract commit block height',
      //   task: async (ctx, task) => {
      //     config.set('platform.masternodeRewardShares.contract.blockHeight', 42);
      //
      //     // eslint-disable-next-line no-param-reassign
      //     task.output = `Reward Share contract block height: ${42}`;
      //   },
      //   options: { persistentOutput: true },
      // },
      {
        title: 'Disconnect SDK',
        task: async (ctx) => ctx.client.disconnect(),
      },
    ]);
  }

  return initTask;
}

module.exports = initTaskFactory;
