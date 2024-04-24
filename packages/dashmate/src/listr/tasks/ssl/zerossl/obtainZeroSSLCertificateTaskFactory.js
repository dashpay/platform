import { Listr } from 'listr2';

import chalk from 'chalk';
import path from 'path';
import fs from 'fs';
import wait from '../../../../util/wait.js';

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
) {
  /**
   * @typedef {obtainZeroSSLCertificateTask}
   * @param {Config} config
   * @return {Promise<Listr>}
   */
  async function obtainZeroSSLCertificateTask(config) {
    // Make sure that required config options are set
    const apiKey = config.get('platform.gateway.ssl.providerConfigs.zerossl.apiKey', true);
    const externalIp = config.get('externalIp', true);

    const sslConfigDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl');
    const csrFilePath = path.join(sslConfigDir, 'csr.pem');
    const privateKeyFilePath = path.join(sslConfigDir, 'private.key');
    const bundleFilePath = path.join(sslConfigDir, 'bundle.crt');

    // Ensure we have config dir created
    fs.mkdirSync(sslConfigDir, { recursive: true });

    return new Listr([
      {
        title: 'Check if certificate already exists and not expiring soon',
        // Skips the check if force flag is set
        skip: (ctx) => ctx.force,
        task: async (ctx, task) => {
          const certificateId = await config.get('platform.gateway.ssl.providerConfigs.zerossl.id');

          if (!certificateId) {
            // Certificate is not configured

            // eslint-disable-next-line no-param-reassign
            task.output = 'Certificate is not configured yet, creating a new one';

            return;
          }

          // Certificate is already configured

          // Check if certificate files are present
          ctx.isCrtFilePresent = fs.existsSync(csrFilePath);

          ctx.isPrivateKeyFilePresent = fs.existsSync(privateKeyFilePath);

          ctx.isBundleFilePresent = fs.existsSync(bundleFilePath);

          // This function will throw an error if certificate with specified ID is not present
          const certificate = await getCertificate(apiKey, certificateId);

          // If certificate exists but private key does not, then we can't setup TLS connection
          // In this case we need to regenerate certificate or put back this private key
          if (!ctx.isPrivateKeyFilePresent) {
            throw new Error(`Certificate private key file not found in ${privateKeyFilePath}.\n`
              + 'Please regenerate the certificate using the the obtain'
              + ' command with the --force flag, and revoke the previous certificate in'
              + ' the ZeroSSL dashboard');
          }

          // We need to make sure that external IP and certificate IP match
          if (certificate.common_name !== externalIp) {
            throw new Error(`Certificate IPe ${certificate.common_name} does not match external IP ${externalIp}.\n`
            + 'Please change the external IP in config or regenerate the certificate '
            + ' using the obtain command with the --force flag, and revoke the previous'
            + ' certificate in the ZeroSSL dashboard');
          }

          if (!certificate.isExpiredInDays(ctx.expirationDays)) {
            // Certificate is not going to expire soon

            if (certificate.status === 'issued') {
              // Certificate is valid, so we might need only to download certificate bundle
              ctx.certificate = certificate;

              // eslint-disable-next-line no-param-reassign
              task.output = `Certificate is valid and expires at ${certificate.expires}`;
            } else if (['pending_validation', 'draft'].includes(certificate.status)) {
              // Certificate is already created, so we just need to pass validation
              // and download certificate file
              ctx.certificate = certificate;

              // We need to download new certificate bundle
              ctx.isBundleFilePresent = false;

              // eslint-disable-next-line no-param-reassign
              task.output = 'Certificate was already created, but not validated yet.';
            } else {
              // Certificate is not valid, so we need to re-create it

              // We need to download certificate bundle
              ctx.isBundleFilePresent = false;

              if (!ctx.isCrtFilePresent) {
                throw new Error(`Certificate request file not found in ${csrFilePath}.\n`
                  + 'To create a new certificate, please use the obtain'
                  + ' command with the --force flag and revoke the previous certificate'
                  + ' in the ZeroSSL dashboard');
              }

              ctx.csr = fs.readFileSync(csrFilePath, 'utf8');

              // eslint-disable-next-line no-param-reassign
              task.output = 'Certificate is not valid. Create a new one';
            }
          } else {
            // Certificate is going to expire soon, we need to obtain a new one

            // We need to download new certificate bundle
            ctx.isBundleFilePresent = false;

            if (!ctx.isCrtFilePresent) {
              throw new Error(`Certificate request file not found in ${csrFilePath}.\n`
                + 'To renew certificate please use the obtain'
                + ' command with the --force flag, and revoke the previous certificate in'
                + ' the ZeroSSL dashboard');
            }

            ctx.csr = fs.readFileSync(csrFilePath, 'utf8');

            // eslint-disable-next-line no-param-reassign
            task.output = `Certificate exists but expires in less than ${ctx.expirationDays} days at ${certificate.expires}. Obtain a new one`;
          }
        },
      },
      {
        title: 'Generate a keypair',
        enabled: (ctx) => !ctx.isCrtFilePresent,
        task: async (ctx) => {
          ctx.keyPair = await generateKeyPair();
          ctx.privateKeyFile = ctx.keyPair.privateKey;
        },
      },
      {
        title: 'Generate certificate request',
        enabled: (ctx) => !ctx.isCrtFilePresent,
        task: async (ctx) => {
          ctx.csr = await generateCsr(
            ctx.keyPair,
            externalIp,
          );
        },
      },
      {
        title: 'Create a certificate',
        skip: (ctx) => ctx.certificate,
        task: async (ctx) => {
          ctx.certificate = await createZeroSSLCertificate(
            ctx.csr,
            externalIp,
            apiKey,
          );

          config.set('platform.gateway.ssl.enabled', true);
          config.set('platform.gateway.ssl.provider', 'zerossl');
          config.set('platform.gateway.ssl.providerConfigs.zerossl.id', ctx.certificate.id);
        },
      },
      {
        title: 'Set up verification server',
        skip: (ctx) => ctx.certificate && !['pending_validation', 'draft'].includes(ctx.certificate.status),
        task: async (ctx) => {
          const validationResponse = ctx.certificate.validation.other_methods[externalIp];

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
          do {
            try {
              await verifyDomain(ctx.certificate.id, apiKey);
            } catch (e) {
              if (ctx.noRetry !== true) {
                retry = await task.prompt({
                  type: 'toggle',
                  header: chalk`  An error occurred during verification: {red ${e.message}}

    Please ensure that port 80 on your public IP address ${externalIp} is open
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
        title: 'Download certificate file',
        skip: (ctx) => ctx.isBundleFilePresent,
        task: async (ctx, task) => {
          for (let retry = 0; retry <= 50; retry += 1) {
            await wait(5000);

            try {
              ctx.certificateFile = await downloadCertificate(
                ctx.certificate.id,
                apiKey,
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
          fs.writeFileSync(privateKeyFilePath, ctx.privateKeyFile, 'utf8');

          // eslint-disable-next-line no-param-reassign
          task.output = privateKeyFilePath;
        },
      },
      {
        title: 'Save certificate request file',
        enabled: (ctx) => !ctx.isCrtFilePresent,
        task: async (ctx, task) => {
          fs.writeFileSync(csrFilePath, ctx.csr, 'utf8');

          // eslint-disable-next-line no-param-reassign
          task.output = csrFilePath;
        },
      },
      {
        title: 'Save certificate file',
        skip: (ctx) => ctx.isBundleFilePresent,
        task: async (ctx, task) => {
          fs.writeFileSync(bundleFilePath, ctx.certificateFile, 'utf8');

          // eslint-disable-next-line no-param-reassign
          task.output = bundleFilePath;
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
