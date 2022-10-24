const { Listr } = require('listr2');
const fs = require('fs');
const path = require('path');
const { HOME_DIR_PATH } = require('../../../../constants');

/**
 *
 * @param {createZeroSSLCertificate} createZeroSSLCertificate
 * @param {verifyDomain} verifyDomain
 * @param {downloadCertificate} downloadCertificate
 * @param {listCertificates} listCertificates
 * @param {saveCertificateTask} saveCertificateTask
 * @param {VerificationServer} verificationServer
 * @return {renewZeroSSLCertificateTask}
 */
function renewZeroSSLCertificateTaskFactory(
  createZeroSSLCertificate,
  verifyDomain,
  downloadCertificate,
  listCertificates,
  saveCertificateTask,
  verificationServer,
) {
  /**
   * @typedef {renewZeroSSLCertificateTask}
   * @param {Config} config
   * @return {Promise<Listr>}
   */
  async function renewZeroSSLCertificateTask(config) {
    return new Listr([
      {
        title: 'Request certificate challenge',
        task: async (ctx) => {
          const crtFile = path.join(HOME_DIR_PATH, 'ssl', config.getName(), 'bundle.crt');

          ctx.csr = fs.readFileSync(crtFile, 'utf8');

          ctx.response = await createZeroSSLCertificate(ctx.csr, config);
        },
      },
      {
        title: 'Set up verification server',
        task: async (ctx) => {
          const validationResponse = ctx.response.validation.other_methods[config.get('externalIp')];
          const route = validationResponse.file_validation_url_http.replace(`http://${config.get('externalIp')}`, '');
          const body = validationResponse.file_validation_content.join('\\n');

          await verificationServer.setup(config, route, body);
        },
      },
      {
        title: 'Start verification server',
        task: async () => verificationServer.start(),
      },
      {
        title: 'Verify IP',
        task: async (ctx) => verifyDomain(ctx.response.id, config.get('platform.dapi.envoy.ssl.providerConfigs.zerossl.apiKey')),
      },
      {
        title: 'Download certificate',
        task: async (ctx) => {
          ctx.certificate = await downloadCertificate(ctx.response.id, config.get('platform.dapi.envoy.ssl.providerConfigs.zerossl.apiKey'));
        },
      },
      {
        title: 'Save certificate',
        task: async () => saveCertificateTask(config),
      },
      {
        title: 'Stop verification server',
        task: async () => {
          await verificationServer.stop();
          await verificationServer.destroy();
        },
      },
    ]);
  }

  return renewZeroSSLCertificateTask;
}

module.exports = renewZeroSSLCertificateTaskFactory;
