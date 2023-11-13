import { Listr }  from 'listr2';

import featureFlagTypes from '@dashevo/feature-flags-contract/lib/featureFlagTypes'
import {ConfigBaseCommand} from "../../oclif/command/ConfigBaseCommand.js";
import {MuteOneLineError} from "../../oclif/errors/MuteOneLineError.js";

export class FeatureFlagCommand extends ConfigBaseCommand {
  static description = 'Register feature flags';

  static flags = {
    ...ConfigBaseCommand.flags,
  };

  static args = [{
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
    }]

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
