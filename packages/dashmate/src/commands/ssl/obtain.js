const { Listr } = require('listr2');
const { Flags } = require('@oclif/core');

const MuteOneLineError = require('../../oclif/errors/MuteOneLineError');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const { EXPIRATION_LIMIT_DAYS } = require('../../ssl/zerossl/Certificate');

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
      'no-retry': noRetry,
      'expiration-days': expirationDays,
      force,
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
        removeEmptyLines: false,
      },
    });

    try {
      await tasks.run({
        noRetry,
        force,
        expirationDays,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

ObtainCommand.description = `Obtain SSL certificate

Create a new SSL certificate or download an already existing one using ZeroSSL as provider
Certificate will be renewed if it is about to expire (see 'expiration-days' flag)
`;

ObtainCommand.flags = {
  ...ConfigBaseCommand.flags,
  verbose: Flags.boolean({ char: 'v', description: 'use verbose mode for output', default: false }),
  'no-retry': Flags.boolean({ description: 'do not retry on IP verification failure', default: false }),
  force: Flags.boolean({ description: 'renew even if certificate is valid', default: false }),
  'expiration-days': Flags.integer({
    description: 'renew if certificate expires within the'
      + ' specified number of days',
    default: EXPIRATION_LIMIT_DAYS,
  }),
};

module.exports = ObtainCommand;
