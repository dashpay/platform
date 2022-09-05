const { Listr } = require('listr2');
const fs = require('fs');

/**
 * @param {generateCsr} generateCsr
 * @param {generateKeyPair} generateKeyPair
 * @param {createCertificate} createCertificate
 * @param {setupVerificationServerTask} setupVerificationServerTask
 * @param {verifyDomain} verifyDomain
 * @param {downloadCertificate} downloadCertificate
 * @param {setupCertificateTask} setupCertificateTask
 * @return {obtainZeroSSLCertificateTask}
 */
function obtainZeroSSLCertificateTaskFactory(
  generateCsr,
  generateKeyPair,
  createCertificate,
  setupVerificationServerTask,
  verifyDomain,
  downloadCertificate,
  setupCertificateTask,
) {
  /**
   * @typedef {obtainZeroSSLCertificateTask}
   * @param config
   * @return {Promise<Listr>}
   */
  async function obtainZeroSSLCertificateTask(config) {
    return new Listr([
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

          fs.rmSync(ctx.envoyConfig, {force: true});
        },
      }]);
  }

  return obtainZeroSSLCertificateTask;
}

module.exports = obtainZeroSSLCertificateTaskFactory;
