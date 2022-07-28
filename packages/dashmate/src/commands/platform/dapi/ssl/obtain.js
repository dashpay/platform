const { Listr } = require('listr2');

const BaseCommand = require('../../../../oclif/command/BaseCommand');
const MuteOneLineError = require('../../../../oclif/errors/MuteOneLineError');

class ObtainCommand extends BaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {homeDirPath} homeDirPath
   * @param {checkCertificate} checkCertificateTask
   * @param {generateCsr} generateCsr
   * @param {createZerosslCertificateTask} createZerosslCertificateTask
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    flags,
    homeDirPath,
    checkCertificateTask,
    generateCsr,
    createZerosslCertificateTask,
    config,
  ) {
    const tasks = new Listr([
      {
        title: 'Check for existing certificate',
        task: async (ctx) => {
          // eslint-disable-next-line no-param-reassign
          ctx.certStatus = await checkCertificateTask('bundle.crt');
        },
      },
      {
        title: 'Check for existing CSR',
        enabled: (ctx) => !ctx.certStatus,
        task: async (ctx) => {
          // eslint-disable-next-line no-param-reassign
          ctx.csrStatus = await checkCertificateTask('domain.csr');
        },
      },
      {
        title: 'Generate CSR',
        enabled: (ctx) => (!ctx.csrStatus && !ctx.certStatus),
        task: () => generateCsr(config.get('externalIp'), homeDirPath),
      },
      {
        title: `Create ZeroSSL cert for IP ${config.get('externalIp')}`,
        enabled: (ctx) => !ctx.certStatus,
        task: () => createZerosslCertificateTask(config),
      },
      {
        title: 'Enable cert in config',
        task: () => {
          config.set('platform.dapi.nginx.ssl.enable', true);
        },
      },
    ],
    {
      rendererOptions: {
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
  ...BaseCommand.flags,
};

module.exports = ObtainCommand;
