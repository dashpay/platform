const { Listr } = require('listr2');

const Dash = require('dash');
const Identifier = require('@dashevo/dpp/lib/Identifier');

/**
 *
 * @return {featureFlagTask}
 */
function featureFlagTaskFactory() {
  /**
   * @typedef {featureFlagTask}
   * @param {Config} config
   * @return {Listr}
   */
  function featureFlagTask(
    config,
  ) {
    return new Listr([
      {
        title: 'Initialize SDK',
        task: async (ctx) => {
          const clientOpts = {
            network: config.get('network'),
          };

          if (ctx.dapiAddress) {
            clientOpts.dapiAddresses = [ctx.dapiAddress];
          }

          ctx.client = new Dash.Client({
            ...clientOpts,
            wallet: {
              HDPrivateKey: ctx.hdPrivateKey,
            },
          });

          const featureFlagsContractId = config.get('platform.featureFlags.contract.id');

          const featureFlagsContract = await ctx.client.platform.contracts.get(
            featureFlagsContractId,
          );

          ctx.client.getApps().set('featureFlags', {
            contractId: Identifier.from(featureFlagsContractId),
            contract: featureFlagsContract,
          });
        },
      },
      {
        title: 'Enable feature flag',
        task: async (ctx) => {
          const featureFlagsFlag = `featureFlags.${ctx.featureFlagName}`;

          const ownerIdentityId = config.get('platform.featureFlags.ownerId');

          const ownerIdentity = await ctx.client.platform.identities.get(ownerIdentityId);

          const featureFlagDocument = await ctx.client.platform.documents.create(
            featureFlagsFlag,
            ownerIdentity,
            {
              enabled: true,
              enableAtHeight: Number(ctx.height),
            },
          );

          // Sign and submit the document(s)
          await ctx.client.platform.documents.broadcast({
            create: [featureFlagDocument],
          }, ownerIdentity);
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Disconnect SDK',
        task: async (ctx) => ctx.client.disconnect(),
      },
    ]);
  }

  return featureFlagTask;
}

module.exports = featureFlagTaskFactory;
