const { Listr } = require('listr2');

const chalk = require('chalk');
const { EXPIRATION_LIMIT_DAYS } = require('../../../../ssl/zerossl/Certificate');

/**
 * @param {generateCsr} generateCsr
 * @param {generateKeyPair} generateKeyPair
 * @param {createZeroSSLCertificate} createZeroSSLCertificate
 * @param {verifyDomain} verifyDomain
 * @param {downloadCertificate} downloadCertificate
 * @param {getCertificate} getCertificate
 * @param {listCertificates} listCertificates
 * @param {saveCertificateTask} saveCertificateTask
 * @param {VerificationServer} verificationServer
 * @return {obtainZeroSSLCertificateTask}
 */
function obtainZeroSSLCertificateTaskFactory(
  generateCsr,
  generateKeyPair,
  createZeroSSLCertificate,
  verifyDomain,
  downloadCertificate,
  getCertificate,
  listCertificates,
  saveCertificateTask,
  verificationServer,
  configFileRepository,
  configFile,
) {
  /**
   * @typedef {obtainZeroSSLCertificateTask}
   * @param {Config} config
   * @return {Promise<Listr>}
   */
  async function obtainZeroSSLCertificateTask(config) {
    // Make sure that required config options are set
    config.get('platform.dapi.envoy.ssl.providerConfigs.zerossl.apiKey', true);
    config.get('externalIp', true);

    return new Listr([
      {
        title: 'Check if certificate already exists and not expiring soon',
        // Skips the check if force flag is set
        skip: (ctx) => ctx.force,
        task: async (ctx, task) => {
          ctx.apiKey = config.get('platform.dapi.envoy.ssl.providerConfigs.zerossl.apiKey');

          const certificateId = await config.get('platform.dapi.envoy.ssl.providerConfigs.zerossl.id');

          // Certificate is already configured
          if (certificateId) {
            const certificate = await getCertificate(ctx.apiKey, certificateId);

            // Certificate is not going to expire soon
            if (!certificate.isExpiredInDays(ctx.expirationDays)) {
              // Certificate is already created, so we just need to pass validation
              // if status is draft or pending_validation and download certificate file
              if (['issued', 'pending_validation', 'draft'].includes(ctx.certificate.status)) {
                ctx.certificate = certificate;

                ctx.privateKeyFile = config.get('platform.dapi.envoy.ssl.providerConfigs.zerossl.privateKey', true);

                // eslint-disable-next-line no-param-reassign
                task.output = `Certificate already exists and expires at ${ctx.certificate.expires}`;
              }
            // Certificate is going to expire soon, we need to obtain a new one
            } else {
              // eslint-disable-next-line no-param-reassign
              task.output = `Certificate exists but expires in less than ${EXPIRATION_LIMIT_DAYS} days at ${ctx.certificate.expires}. Obtain a new one`;
            }
          }
        },
      },
      {
        title: 'Generate a keypair',
        skip: (ctx) => ctx.certificate,
        task: async (ctx) => {
          ctx.keyPair = await generateKeyPair();
          ctx.privateKeyFile = ctx.keyPair.privateKey;
        },
      },
      {
        title: 'Generate CSR',
        skip: (ctx) => ctx.certificate,
        task: async (ctx) => {
          ctx.csr = await generateCsr(
            ctx.keyPair,
            config.get('externalIp'),
          );
        },
      },
      {
        title: 'Create a certificate',
        skip: (ctx) => ctx.certificate,
        task: async (ctx) => {
          ctx.certificate = await createZeroSSLCertificate(
            ctx.csr,
            config.get('externalIp'),
            ctx.apiKey,
          );

          config.set('platform.dapi.envoy.ssl.provider', 'zerossl');
          config.set('platform.dapi.envoy.ssl.providerConfigs.zerossl.id', ctx.certificate.id);
          config.set('platform.dapi.envoy.ssl.providerConfigs.zerossl.privateKey', ctx.privateKeyFile);
        },
      },
      {
        title: 'Set up verification server',
        skip: (ctx) => !['pending_validation', 'draft'].includes(ctx.certificate.status),
        task: async (ctx) => {
          const validationResponse = ctx.certificate.validation.other_methods[config.get('externalIp')];
          const route = validationResponse.file_validation_url_http.replace(`http://${config.get('externalIp')}`, '');
          const body = validationResponse.file_validation_content.join('\\n');

          await verificationServer.setup(config, route, body);
        },
      },
      {
        title: 'Start verification server',
        skip: (ctx) => !['pending_validation', 'draft'].includes(ctx.certificate.status),
        task: async () => verificationServer.start(),
      },
      {
        title: 'Verify IP',
        skip: (ctx) => !['pending_validation', 'draft'].includes(ctx.certificate.status),
        task: async (ctx, task) => {
          let retry;
          do {
            try {
              await verifyDomain(ctx.certificate.id, ctx.apiKey);
            } catch (e) {
              if (ctx.noRetry !== true) {
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
              }

              if (!retry) {
                throw e;
              }
            }
          } while (retry);
        },
      },
      {
        title: 'Download certificate files',
        task: async (ctx) => {
          ctx.certificateFile = await downloadCertificate(
            ctx.certificate.id,
            ctx.apiKey,
          );
        },
      },
      {
        title: 'Save certificate files',
        task: async () => saveCertificateTask(config),
      },
      {
        title: 'Stop verification server',
        skip: (ctx) => !['pending_validation', 'draft'].includes(ctx.certificate.status),
        task: async () => {
          await verificationServer.stop();
          await verificationServer.destroy();
        },
      },
    ]);
  }

  return obtainZeroSSLCertificateTask;
}

module.exports = obtainZeroSSLCertificateTaskFactory;
