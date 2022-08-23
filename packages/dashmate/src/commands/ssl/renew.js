const { Listr } = require('listr2');
const { Flags } = require('@oclif/core');

const fs = require('fs');
const path = require('path');
const MuteOneLineError = require('../../oclif/errors/MuteOneLineError');

const listCertificates = require('../../ssl/zerossl/listCertificates');
const downloadCertificate = require('../../ssl/zerossl/downloadCertificate');
const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const { HOME_DIR_PATH } = require('../../constants');

class RenewCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {Config} config
   * @param {setupVerificationServerTask} setupVerificationServerTask
   * @param {createCertificate} createCertificate
   * @param {verifyDomain} verifyDomain
   * @param {setupCertificateTask} setupCertificateTask
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      verbose: isVerbose,
    },
    config,
    setupVerificationServerTask,
    createCertificate,
    verifyDomain,
    setupCertificateTask,
  ) {
    const tasks = new Listr([
      {
        title: `Search ZeroSSL cert for ip ${config.get('externalIp')}`,
        task: async (ctx, task) => {
          const response = await listCertificates(config.get('platform.dapi.envoy.ssl.zerossl.apikey'));

          const certificate = response.results.find((result) => result.common_name === config.get('externalIp'));

          if (!certificate) {
            throw new Error('There is no certificate to renew');
          }

          ctx.certId = certificate.id;
          ctx.response = certificate;

          // eslint-disable-next-line no-param-reassign
          task.output = `Cert found: ${ctx.certId}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Request certificate challenge',
        task: async (ctx) => {
          const crtFile = path.join(HOME_DIR_PATH, 'ssl', config.getName(), 'bundle.crt');

          ctx.csr = fs.readFileSync(crtFile, 'utf-8');

          ctx.response = await createCertificate(ctx.csr, config);
        },
      },
      {
        title: 'Set up verification server',
        task: async () => setupVerificationServerTask(config),
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

RenewCommand.description = `Renew SSL Cert
...
Renew SSL Cert using ZeroSLL API Key
`;

RenewCommand.flags = {
  ...ConfigBaseCommand.flags,
  verbose: Flags.boolean({ char: 'v', description: 'use verbose mode for output', default: false }),
};

module.exports = RenewCommand;
