const { Listr } = require('listr2');
const { Flags } = require('@oclif/core');

const MuteOneLineError = require('../../oclif/errors/MuteOneLineError');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');

class ObtainCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {Config} config
   * @param {obtainZeroSSLCertificateTask} obtainZeroSSLCertificateTask
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      verbose: isVerbose,
    },
    config,
    obtainZeroSSLCertificateTask,
  ) {
    const tasks = new Listr([
      {
        title: 'Obtain ZeroSSL certificate',
        task: async () => obtainZeroSSLCertificateTask(config),
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
      await tasks.run();
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

ObtainCommand.description = `Obtain SSL certificate

Obtain SSL certificate using ZeroSSL API Key
`;

ObtainCommand.flags = {
  ...ConfigBaseCommand.flags,
  verbose: Flags.boolean({ char: 'v', description: 'use verbose mode for output', default: false }),
};

module.exports = ObtainCommand;
