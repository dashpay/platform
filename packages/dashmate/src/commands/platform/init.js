const { Listr } = require('listr2');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const MuteOneLineError = require('../../oclif/errors/MuteOneLineError');

class InitCommand extends ConfigBaseCommand {
  /**
   *
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @param {initTask} initTask
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    {
      'dapi-address': dapiAddress,
      'funding-private-key': fundingPrivateKeyString,
    },
    {
      verbose: isVerbose,
    },
    dockerCompose,
    initTask,
    config,
  ) {
    const tasks = new Listr([
      {
        title: 'Initialize Platform',
        task: () => initTask(config),
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
        fundingPrivateKeyString,
        dapiAddress,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

InitCommand.description = `Initialize platform
...
Register DPNS Contract and "dash" top-level domain
`;

InitCommand.args = [{
  name: 'funding-private-key',
  required: true,
  description: 'private key with dash for funding account',
},
{
  name: 'dapi-address',
  required: false,
  description: 'DAPI address to send init transitions to',
}];

InitCommand.flags = {
  ...ConfigBaseCommand.flags,
};

module.exports = InitCommand;
