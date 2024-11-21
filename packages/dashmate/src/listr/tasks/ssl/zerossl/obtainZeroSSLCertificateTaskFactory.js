import { Listr } from 'listr2';

import chalk from 'chalk';
import fs from 'fs';
import lodash from 'lodash';
import wait from '../../../../util/wait.js';
import { ERRORS } from '../../../../ssl/zerossl/validateZeroSslCertificateFactory.js';

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
 * @param {HomeDir} homeDir
 * @param {validateZeroSslCertificate} validateZeroSslCertificate
 * @param {ConfigFileJsonRepository} configFileRepository
 * @param {ConfigFile} configFile
 * @return {obtainZeroSSLCertificateTask}
 */
export default function obtainZeroSSLCertificateTaskFactory(
  generateCsr,
  generateKeyPair,
  createZeroSSLCertificate,
  verifyDomain,
  downloadCertificate,
  getCertificate,
  listCertificates,
  saveCertificateTask,
  verificationServer,
  homeDir,
  validateZeroSslCertificate,
  configFileRepository,
  configFile,
) {
  /**
   * @typedef {obtainZeroSSLCertificateTask}
   * @param {Config} config
   * @return {Listr}
   */
  function obtainZeroSSLCertificateTask(config) {
    return new Listr([
      {
        title: 'Check if certificate already exists and not expiring soon',
        // Skips the check if force flag is set
        skip: (ctx) => ctx.force,
        task: async (ctx, task) => {
          const { error, data } = await validateZeroSslCertificate(config, ctx.expirationDays);

          lodash.merge(ctx, data);

          // Ensure we have config dir created
          fs.mkdirSync(ctx.sslConfigDir, { recursive: true });

          switch (error) {
            case undefined:
              // eslint-disable-next-line no-param-reassign
              task.output = `Certificate is valid and expires at ${ctx.certificate.expires}`;
              break;
            case ERRORS.API_KEY_IS_NOT_SET:
              throw new Error('ZeroSSL API key is not set. Please set it in the config file');
            case ERRORS.EXTERNAL_IP_IS_NOT_SET:
              throw new Error('External IP is not set. Please set it in the config file');
            case ERRORS.CERTIFICATE_ID_IS_NOT_SET:
              // eslint-disable-next-line no-param-reassign
              task.output = 'Certificate is not configured yet, creating a new one';

              // We need to create a new certificate
              ctx.certificate = null;
              break;
            case ERRORS.PRIVATE_KEY_IS_NOT_PRESENT:
              // If certificate exists but private key does not, then we can't set up TLS connection
              // In this case we need to regenerate certificate or put back this private key
              throw new Error(`Certificate private key file not found in ${ctx.privateKeyFilePath}.\n`
                + 'Please regenerate the certificate using the obtain'
                + ' command with the --force flag and revoke the previous certificate in'
                + ' the ZeroSSL dashboard');
            case ERRORS.EXTERNAL_IP_MISMATCH:
              throw new Error(`Certificate IPe ${ctx.certificate.common_name} does not match external IP ${ctx.externalIp}.\n`
                + 'Please change the external IP in config. Otherwise, regenerate the certificate '
                + ' using the obtain command with the --force flag and revoke the previous'
                + ' certificate in the ZeroSSL dashboard');
            case ERRORS.CSR_FILE_IS_NOT_PRESENT:
              throw new Error(`Certificate request file not found in ${ctx.csrFilePath}.\n`
                + 'To renew certificate please use the obtain'
                + ' command with the --force flag, and revoke the previous certificate in'
                + ' the ZeroSSL dashboard');
            case ERRORS.CERTIFICATE_EXPIRES_SOON:
              // eslint-disable-next-line no-param-reassign
              task.output = `Certificate exists but expires in less than ${ctx.expirationDays} days at ${ctx.certificate.expires}. Obtain a new one`;

              // We need to create a new certificate
              ctx.certificate = null;
              break;
            case ERRORS.CERTIFICATE_IS_NOT_VALIDATED:
              // eslint-disable-next-line no-param-reassign
              task.output = 'Certificate was already created, but has not been validated yet.';
              break;
            case ERRORS.CERTIFICATE_IS_NOT_VALID:
              // eslint-disable-next-line no-param-reassign
              task.output = 'Certificate is not valid. Create a new one';

              // We need to create a new certificate
              ctx.certificate = null;
              break;
            case ERRORS.ZERO_SSL_API_ERROR:
              throw ctx.error;
            default:
              throw new Error(`Unknown error: ${error}`);
          }
        },
      },
      {
        title: 'Generate a keypair',
        enabled: (ctx) => !ctx.isCsrFilePresent,
        task: async (ctx) => {
          ctx.keyPair = await generateKeyPair();
          ctx.privateKeyFile = ctx.keyPair.privateKey;
        },
      },
      {
        title: 'Generate certificate request',
        enabled: (ctx) => !ctx.isCsrFilePresent,
        task: async (ctx) => {
          ctx.csr = await generateCsr(
            ctx.keyPair,
            ctx.externalIp,
          );
        },
      },
      {
        title: 'Create a certificate',
        skip: (ctx) => ctx.certificate,
        task: async (ctx) => {
          ctx.certificate = await createZeroSSLCertificate(
            ctx.csr,
            ctx.externalIp,
            ctx.apiKey,
          );

          config.set('platform.gateway.ssl.enabled', true);
          config.set('platform.gateway.ssl.provider', 'zerossl');
          config.set('platform.gateway.ssl.providerConfigs.zerossl.id', ctx.certificate.id);

          // Save config file
          configFileRepository.write(configFile);
        },
      },
      {
        title: 'Set up verification server',
        skip: (ctx) => ctx.certificate && !['pending_validation', 'draft'].includes(ctx.certificate.status),
        task: async (ctx) => {
          const validationResponse = ctx.certificate.validation.other_methods[ctx.externalIp];

          await verificationServer.setup(
            config,
            validationResponse.file_validation_url_http,
            validationResponse.file_validation_content,
          );
        },
      },
      {
        title: 'Start verification server',
        skip: (ctx) => ctx.certificate && !['pending_validation', 'draft'].includes(ctx.certificate.status),
        task: async () => verificationServer.start(),
      },
      {
        title: 'Verify certificate IP address',
        skip: (ctx) => ctx.certificate && !['pending_validation', 'draft'].includes(ctx.certificate.status),
        task: async (ctx, task) => {
          let retry;
          let autoRetryCount = 0;
          const MAX_AUTO_RETRIES = 3; // Adjust based on requirements
          do {
            try {
              await verifyDomain(ctx.certificate.id, ctx.apiKey);
            } catch (e) {
              // Error: The given certificate is not ready for domain verification
              // Sometimes this error means that certificate is already verified
              if (e.code === 2831) {
                const certificate = await getCertificate(ctx.apiKey, ctx.certificate.id);
                // Just proceed on certificate download if we see it's already issued.
                if (certificate.status === 'issued') {
                  return;
                }
              }

              if (e.type === 'domain_control_validation_failed') {
                // Retry on this undocumented error whatever it means
                if (autoRetryCount >= MAX_AUTO_RETRIES) {
                  throw e;
                }
                autoRetryCount++;
                if (process.env.DEBUG) {
                  // eslint-disable-next-line no-console
                  console.warn(`Retry ${autoRetryCount}/${MAX_AUTO_RETRIES} verification due to domain_control_validation_failed error`);
                }
                await wait(5000);
              } else {
                if (ctx.noRetry !== true) {
                  retry = await task.prompt({
                    type: 'toggle',
                    header: chalk`  An error occurred during verification: {red ${e.message}}

    Please ensure that port 80 on your public IP address ${ctx.externalIp} is open
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
            }
          } while (retry);
        },
      },
      {
        title: 'Download certificate file',
        skip: (ctx) => ctx.isBundleFilePresent,
        task: async (ctx, task) => {
          for (let retry = 0; retry <= 50; retry += 1) {
            await wait(5000);

            try {
              ctx.certificateFile = await downloadCertificate(
                ctx.certificate.id,
                ctx.apiKey,
              );

              // eslint-disable-next-line no-param-reassign
              task.output = 'Successfully downloaded';

              break;
            } catch (e) {
              if (e.code !== 2832) {
                throw e;
              }

              // eslint-disable-next-line no-param-reassign
              task.output = 'Certificate is not ready yet. Waiting...';
            }
          }

          if (!ctx.certificateFile) {
            throw new Error('Certificate is not ready yet. Please try again later');
          }
        },
      },
      {
        title: 'Save certificate private key file',
        enabled: (ctx) => !ctx.isPrivateKeyFilePresent,
        task: async (ctx, task) => {
          fs.writeFileSync(ctx.privateKeyFilePath, ctx.privateKeyFile, 'utf8');

          // eslint-disable-next-line no-param-reassign
          task.output = ctx.privateKeyFilePath;
        },
      },
      {
        title: 'Save certificate request file',
        enabled: (ctx) => !ctx.isCsrFilePresent,
        task: async (ctx, task) => {
          fs.writeFileSync(ctx.csrFilePath, ctx.csr, 'utf8');

          // eslint-disable-next-line no-param-reassign
          task.output = ctx.csrFilePath;
        },
      },
      {
        title: 'Save certificate file',
        skip: (ctx) => ctx.isBundleFilePresent,
        task: async (ctx, task) => {
          fs.writeFileSync(ctx.bundleFilePath, ctx.certificateFile, 'utf8');

          // eslint-disable-next-line no-param-reassign
          task.output = ctx.bundleFilePath;
        },
      },
      {
        title: 'Stop verification server',
        skip: (ctx) => ctx.certificate && !['pending_validation', 'draft'].includes(ctx.certificate.status),
        task: async () => {
          await verificationServer.stop();
          await verificationServer.destroy();
        },
      },
    ], {
      rendererOptions: {
        showErrorMessage: true,
      },
    });
  }

  return obtainZeroSSLCertificateTask;
}
