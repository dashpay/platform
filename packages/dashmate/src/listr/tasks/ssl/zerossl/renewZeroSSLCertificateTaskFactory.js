const { Listr } = require('listr2');

const chalk = require('chalk');

/**
 *
 * @param {createZeroSSLCertificate} createZeroSSLCertificate
 * @param {verifyDomain} verifyDomain
 * @param {downloadCertificate} downloadCertificate
 * @param {listCertificates} listCertificates
 * @param {saveCertificateTask} saveCertificateTask
 * @param {VerificationServer} verificationServer
 * @param {generateKeyPair} generateKeyPair
 * @param {generateCsr} generateCsr
 * @return {renewZeroSSLCertificateTask}
 */
function renewZeroSSLCertificateTaskFactory(
  createZeroSSLCertificate,
  verifyDomain,
  downloadCertificate,
  listCertificates,
  saveCertificateTask,
  verificationServer,
  generateKeyPair,
  generateCsr,
) {
  /**
   * @typedef {renewZeroSSLCertificateTask}
   * @param {Config} config
   * @return {Promise<Listr>}
   */
  async function renewZeroSSLCertificateTask(config) {
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
          ctx.response = await createZeroSSLCertificate(ctx.csr, config.get('externalIp'), config.get('platform.dapi.envoy.ssl.providerConfigs.zerossl.apiKey'));
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
      // TODO: Duplicate tasks
      {
        title: 'Verify IP',
        task: async (ctx, task) => {
          let retry;
          do {
            try {
              await verifyDomain(ctx.response.id, config.get('platform.dapi.envoy.ssl.providerConfigs.zerossl.apiKey'));
            } catch (e) {
              retry = await task.prompt({
                type: 'toggle',
                header: chalk`  An error occurred during verification: {red ${e.message}}

  Please ensure that port 80 on your public IP address ${config.get('externalIp')} is open
  for incoming HTTP connections. You may need to configure your firewall to
  ensure this port is accessible from the public internet. If you are using
  Network Address Translation (NAT), please enable port forwarding for port 80
  and all Dash service ports listed above.`,
                message: 'Try again?',
                enabled: 'Yes',
                disabled: 'No',
                initial: true,
              });

              if (!retry) {
                throw e;
              }
            }
          } while (retry);
        },
      },
      {
        title: 'Download certificate',
        task: async (ctx) => {
          ctx.certificate = await downloadCertificate(ctx.response.id, config.get('platform.dapi.envoy.ssl.providerConfigs.zerossl.apiKey'));
        },
      },
      {
        title: 'Save certificate',
        task: async (ctx) => {
          config.set('platform.dapi.envoy.ssl.providerConfigs.zerossl.id', ctx.response.id);

          return saveCertificateTask(config);
        },
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
