const { Listr } = require('listr2');

const featureFlagTypes = require('@dashevo/feature-flags-contract/lib/featureFlagTypes');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const MuteOneLineError = require('../../oclif/errors/MuteOneLineError');

class FeatureFlagCommand extends ConfigBaseCommand {
  /**
   *
   * @param {Object} args
   * @param {Object} flags
   * @param {featureFlagTask} featureFlagTask
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    {
      name: featureFlagName,
      height,
      'hd-private-key': hdPrivateKey,
      'dapi-address': dapiAddress,
    },
    {
      verbose: isVerbose,
    },
    featureFlagTask,
    config,
  ) {
    const tasks = new Listr([
      {
        title: 'Initialize Feature Flags',
        task: () => featureFlagTask(config),
      },
    ],
    {
      renderer: isVerbose ? 'verbose' : 'default',
      rendererOptions: {
        showTimer: isVerbose,
        clearOutput: false,
        collapse: false,
        showSubtasks: true,
      },
    });

    try {
      await tasks.run({
        featureFlagName,
        height,
        hdPrivateKey,
        dapiAddress,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

FeatureFlagCommand.description = `Feature flags
...
Register feature flags
`;

FeatureFlagCommand.args = [{
  name: 'name',
  required: true,
  description: 'name of the feature flag to process',
  options: Object.values(featureFlagTypes),
},
{
  name: 'height',
  required: true,
  description: 'height at which feature flag should be enabled',
},
{
  name: 'hd-private-key',
  required: true,
  description: 'feature flag hd private key',
},
{
  name: 'dapi-address',
  required: true,
  description: 'DAPI address to send feature flags transitions to',
}];

FeatureFlagCommand.flags = {
  ...ConfigBaseCommand.flags,
};

module.exports = FeatureFlagCommand;
