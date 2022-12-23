const { Listr } = require('listr2');
const { Flags } = require('@oclif/core');

const MuteOneLineError = require('../../oclif/errors/MuteOneLineError');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');

class RenewCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {Config} config
   * @param {createCertificate} createCertificate
   * @param {verifyDomain} verifyDomain
   * @param {downloadCertificate} downloadCertificate
   * @param {listCertificates} listCertificates
   * @param {saveCertificateTask} saveCertificateTask
   * @param {VerificationServer} verificationServer
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      verbose: isVerbose,
    },
    config,
    renewZeroSSLCertificateTask,
  ) {
    const tasks = new Listr([
      {
        title: 'Renew ZeroSSL certificate',
        task: async () => renewZeroSSLCertificateTask(config),
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

RenewCommand.description = `Renew SSL Cert
...
Renew SSL Cert using ZeroSLL API Key
`;

RenewCommand.flags = {
  ...ConfigBaseCommand.flags,
  verbose: Flags.boolean({ char: 'v', description: 'use verbose mode for output', default: false }),
};

module.exports = RenewCommand;
