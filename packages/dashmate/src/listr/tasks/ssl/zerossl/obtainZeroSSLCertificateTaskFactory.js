import { Listr } from 'listr2';

import chalk from 'chalk';
import fs from 'fs';
import lodash from 'lodash';
import promptOrThrow from '../../../../util/promptOrThrow.js';
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
) {
  /**
   * @typedef {obtainZeroSSLCertificateTask}
   * @param {Config} config
   * @param {Object} options
   * @param {function(): void} options.onCertificateCreated
   * @return {Listr}
   */
  function obtainZeroSSLCertificateTask(config, options = {}) {
    const { onCertificateCreated } = options;

    if (typeof onCertificateCreated !== 'function') {
      throw new TypeError('onCertificateCreated callback is required');
    }

    const configurationUpdateRequired = !config.get('platform.gateway.ssl.enabled')
      || config.get('platform.gateway.ssl.provider') !== 'zerossl';

    const tasks = new Listr([
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
          ctx.createdCertificate = true;
          config.set('platform.gateway.ssl.providerConfigs.zerossl.id', ctx.certificate.id);

          onCertificateCreated();
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
        task: async (ctx) => {
          await verificationServer.start();

          const isResponding = await verificationServer.waitForServerIsResponding();

          if (!isResponding) {
            throw new Error(`Verification server is not responding.
Please ensure that port 80 on your public IP address ${ctx.externalIp} is open
for incoming HTTP connections. You may need to configure your firewall to
ensure this port is accessible from the public internet. If you are using
Network Address Translation (NAT), please enable port forwarding for port 80
and all Dash service ports listed above.`);
          }
        },
      },
      {
        title: 'Verify certificate IP address',
        skip: (ctx) => ctx.certificate && !['pending_validation', 'draft'].includes(ctx.certificate.status),
        task: async (ctx, task) => {
          let retry;
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

              // Prompting needs a positive opt-in from the entry point rather
              // than the absence of noRetry. Gating on noRetry alone prompts
              // unless a caller remembers to say otherwise, and the caller
              // most likely to forget renews certificates unattended inside a
              // container, where a prompt never settles and never releases the
              // config lock it holds.
              if (ctx.noRetry !== true && ctx.interactive === true) {
                let errorMessage = e.message;

                // Get the error message from details if it exists
                if (e.type === 'domain_control_validation_failed' && e.details[ctx.externalIp]) {
                  const errorDetails = Object.values(e.details[ctx.externalIp])[0];
                  if (errorDetails?.error) {
                    errorMessage = errorDetails.error_info;
                  }
                }

                retry = await promptOrThrow(task, {
                  type: 'toggle',
                  header: chalk`  An error occurred during verification: {red ${errorMessage}}

  Please ensure that port 80 on your public IP address ${ctx.externalIp} is open
  for incoming HTTP connections. You may need to configure your firewall to
  ensure this port is accessible from the public internet. If you are using
  Network Address Translation (NAT), please enable port forwarding for port 80
  and all Dash service ports listed above.`,
                  message: 'Try again?',
                  enabled: 'Yes',
                  disabled: 'No',
                  initial: true,
                }, { interactive: ctx.interactive });
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
        task: async (ctx, task) => {
          if (ctx.isPrivateKeyFilePresent) {
            // A key written before Dashmate set a mode is group- and
            // world-readable, and reusing it skips the write that would fix
            // that - so tighten what is already there. An owner that chose
            // something stricter keeps it. Presence was decided by an earlier
            // validation step, so confirm it rather than assume it still holds.
            if (fs.existsSync(ctx.privateKeyFilePath)) {
              // eslint-disable-next-line no-bitwise
              const mode = fs.statSync(ctx.privateKeyFilePath).mode & 0o700;

              fs.chmodSync(ctx.privateKeyFilePath, mode);
            }
          } else {
            fs.writeFileSync(ctx.privateKeyFilePath, ctx.privateKeyFile, {
              encoding: 'utf8',
              mode: 0o600,
            });
            fs.chmodSync(ctx.privateKeyFilePath, 0o600);
          }

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
      {
        title: 'Update configuration',
        enabled: (ctx) => ctx.createdCertificate || configurationUpdateRequired,
        task: async () => {
          config.set('platform.gateway.ssl.enabled', true);
          config.set('platform.gateway.ssl.provider', 'zerossl');
        },
      },
    ], {
      rendererOptions: {
        showErrorMessage: true,
      },
    });

    // Wrap run() to ensure the verification server is always cleaned up on failure.
    // If a task after "Start verification server" throws (e.g. domain verification
    // or certificate download fails), Listr aborts and the "Stop verification server"
    // task at the end never executes — leaving an orphaned container bound to port 80.
    // This wrapper guarantees cleanup regardless of where the pipeline fails.
    const originalRun = tasks.run.bind(tasks);
    tasks.run = async (context) => {
      try {
        return await originalRun(context);
      } catch (error) {
        // Best-effort cleanup — never mask the original error, but do surface a
        // cleanup failure: a stuck verification container left bound to port 80 is
        // exactly the condition this wrapper exists to prevent, so it must be visible.
        try {
          await verificationServer.stop();
        } catch (stopError) {
          // stop() is a no-op when no container was started; a real throw here means
          // the verification container may still be running on port 80.
          // eslint-disable-next-line no-console
          console.error(`Failed to stop verification server during cleanup: ${stopError.message}`);
        }
        try {
          await verificationServer.destroy();
        } catch (destroyError) {
          // destroy() is a no-op when the server was never set up (the pipeline
          // failed before setup), so any throw here is a genuine cleanup failure.
          // eslint-disable-next-line no-console
          console.error(`Failed to destroy verification server during cleanup: ${destroyError.message}`);
        }
        throw error;
      }
    };

    return tasks;
  }

  return obtainZeroSSLCertificateTask;
}
