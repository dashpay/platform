const { Listr } = require('listr2');
const { Flags } = require('@oclif/core');

const fs = require('fs');
const MuteOneLineError = require('../../oclif/errors/MuteOneLineError');
const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');

class ObtainCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {Config} config
   * @param {generateCsr} generateCsr
   * @param {generateKeyPair} generateKeyPair
   * @param {createCertificate} createCertificate
   * @param {setupVerificationServerTask} setupVerificationServerTask
   * @param {verifyDomain} verifyDomain
   * @param {downloadCertificate} downloadCertificate
   * @param {setupCertificateTask} setupCertificateTask
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      verbose: isVerbose,
    },
    config,
    generateCsr,
    generateKeyPair,
    createCertificate,
    setupVerificationServerTask,
    verifyDomain,
    downloadCertificate,
    setupCertificateTask,
  ) {
    const tasks = new Listr([
      {
        title: 'Generate a keypair',
        task: async (ctx) => {
          ctx.keyPair = await generateKeyPair();
        },
      },
      {
        title: 'Generate CSR',
        task: async (ctx) => {
          ctx.csr = await generateCsr(ctx.keyPair, config.get('externalIp', true));
        },
      },
      {
        title: 'Request certificate challenge',
        task: async (ctx) => {
          ctx.response = await createCertificate(ctx.csr, config.get('externalIp'), config.get('platform.dapi.envoy.ssl.zerossl.apikey'));
        },
      },
      {
        title: 'Set up verification server',
        task: async () => setupVerificationServerTask(config),
      },
      {
        title: 'Start verification server',
        task: async (ctx) => ctx.server.start(),
      },
      {
        title: 'Verify IP',
        task: async (ctx) => verifyDomain(ctx.response.id, config.get('platform.dapi.envoy.ssl.zerossl.apikey')),
      },
      {
        title: 'Download certificate',
        task: async (ctx) => {
          ctx.certificate = await downloadCertificate(ctx.response.id, config.get('platform.dapi.envoy.ssl.zerossl.apikey'));
        },
      },
      {
        title: 'Set up certificate',
        task: async () => setupCertificateTask(config),
      },
      {
        title: 'Stop temp server',
        task: async (ctx) => {
          await ctx.envoy.stop();

          fs.rmSync(ctx.envoyConfig, { force: true });
        },
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

ObtainCommand.description = `Obtain SSL Cert
...
Obtain SSL Cert using ZeroSLL API Key
`;

ObtainCommand.flags = {
  ...ConfigBaseCommand.flags,
  verbose: Flags.boolean({ char: 'v', description: 'use verbose mode for output', default: false }),
};

module.exports = ObtainCommand;
