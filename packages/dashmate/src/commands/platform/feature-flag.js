import { Listr } from 'listr2';

import featureFlagTypes from '@dashevo/feature-flags-contract/lib/featureFlagTypes.js';
import { Args } from '@oclif/core';
import ConfigBaseCommand from '../../oclif/command/ConfigBaseCommand.js';
import MuteOneLineError from '../../oclif/errors/MuteOneLineError.js';

export default class FeatureFlagCommand extends ConfigBaseCommand {
  static description = 'Register feature flags';

  static flags = {
    ...ConfigBaseCommand.flags,
  };

  static args = {
    name: Args.string(
      {
        name: 'name',
        required: true,
        description: 'name of the feature flag to process',
        options: Object.values(featureFlagTypes),
      },
    ),
    height: Args.string(
      {
        name: 'height',
        required: true,
        description: 'height at which feature flag should be enabled',
      },
    ),
    'hd-private-key': Args.string(
      {
        name: 'hd-private-key',
        required: true,
        description: 'feature flag hd private key',
      },
    ),
    'dapi-address': Args.string(
      {
        name: 'dapi-address',
        required: true,
        description: 'DAPI address to send feature flags transitions to',
      },
    ),
  };

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
    const tasks = new Listr(
      [
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
      },
    );

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
