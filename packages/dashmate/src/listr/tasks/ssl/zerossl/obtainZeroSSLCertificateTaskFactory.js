import { Listr } from 'listr2';

import chalk from 'chalk';
import fs from 'fs';
import path from 'path';
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
    const tasks = new Listr([
      {
        title: 'Initialize configuration',
        task: async (ctx) => {
          // Always load configuration and paths, even under --force.
          // The existing-certificate check below is the only step --force should
          // skip. Skipping this init left ctx.externalIp/ctx.apiKey undefined,
          // which propagated into generateCsr and crashed node-forge with
          // "Attribute value not specified." See dashpay/platform#3803 / #4249.
          ctx.apiKey = config.get('platform.gateway.ssl.providerConfigs.zerossl.apiKey');

          if (!ctx.apiKey) {
            throw new Error('ZeroSSL API key is not set. Please set it in the config file');
          }

          ctx.externalIp = config.get('externalIp');

          if (!ctx.externalIp) {
            throw new Error('External IP is not set. Please set it in the config file');
          }

          ctx.sslConfigDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl');
          ctx.csrFilePath = path.join(ctx.sslConfigDir, 'csr.pem');
          ctx.privateKeyFilePath = path.join(ctx.sslConfigDir, 'private.key');
          ctx.bundleFilePath = path.join(ctx.sslConfigDir, 'bundle.crt');

          fs.mkdirSync(ctx.sslConfigDir, { recursive: true });

          if (ctx.force) {
            // Force a clean regeneration: ignore any existing keypair, CSR, bundle,
            // or certificate state so the generate/create/save tasks all run.
            ctx.isCsrFilePresent = false;
            ctx.isPrivateKeyFilePresent = false;
            ctx.isBundleFilePresent = false;
            ctx.certificate = null;
          }
        },
      },
      {
        title: 'Check if certificate already exists and not expiring soon',
        // Skips the check if force flag is set
        skip: (ctx) => ctx.force,
        task: async (ctx, task) => {
          const { error, data } = await validateZeroSslCertificate(config, ctx.expirationDays);

          lodash.merge(ctx, data);

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
          // Publish the replacement ID only after its key, CSR, and bundle are ready.
          ctx.isCertificateCreated = true;
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

              // If retry is disabled, throw the error
              // or prompt the user to retry
              if (ctx.noRetry !== true) {
                let errorMessage = e.message;

                // Get the error message from details if it exists
                if (e.type === 'domain_control_validation_failed' && e.details[ctx.externalIp]) {
                  const errorDetails = Object.values(e.details[ctx.externalIp])[0];
                  if (errorDetails?.error) {
                    errorMessage = errorDetails.error_info;
                  }
                }

                retry = await task.prompt({
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
        title: 'Save certificate files and configuration',
        enabled: (ctx) => ctx.isCertificateCreated
          || !ctx.isPrivateKeyFilePresent
          || !ctx.isCsrFilePresent
          || !ctx.isBundleFilePresent,
        task: async (ctx, task) => {
          const artifacts = [
            {
              shouldSave: !ctx.isPrivateKeyFilePresent,
              filePath: ctx.privateKeyFilePath,
              content: ctx.privateKeyFile,
              // Owner-only: renameSync replaces the destination inode, so the
              // staged mode is what the installed private key ends up with
              mode: 0o600,
            },
            {
              shouldSave: !ctx.isCsrFilePresent,
              filePath: ctx.csrFilePath,
              content: ctx.csr,
            },
            {
              shouldSave: !ctx.isBundleFilePresent,
              filePath: ctx.bundleFilePath,
              content: ctx.certificateFile,
            },
          ].filter(({ shouldSave }) => shouldSave);

          const stagingDir = fs.mkdtempSync(path.join(ctx.sslConfigDir, '.zerossl-'));
          const configPaths = [
            'platform.gateway.ssl.enabled',
            'platform.gateway.ssl.provider',
            'platform.gateway.ssl.providerConfigs.zerossl.id',
          ];
          let previousConfig;
          let stagedArtifacts;
          let artifactInstallStarted = false;
          let configWasUpdated = false;

          try {
            stagedArtifacts = artifacts.map(({ filePath, content, mode }) => {
              const stagedFilePath = path.join(stagingDir, path.basename(filePath));
              const wasPresent = fs.existsSync(filePath);
              const previousContent = wasPresent
                ? fs.readFileSync(filePath, 'utf8')
                : undefined;

              fs.writeFileSync(stagedFilePath, content, { encoding: 'utf8', mode });

              return {
                stagedFilePath,
                filePath,
                content,
                wasPresent,
                previousContent,
              };
            });

            artifactInstallStarted = true;
            stagedArtifacts.forEach(({
              stagedFilePath, filePath, content, wasPresent,
            }) => {
              if (wasPresent) {
                // The gateway container bind mounts bundle.crt and private.key
                // as single files, so it stays attached to the mounted inode
                // for its lifetime. Overwrite in place so the renewal SIGHUP
                // hot restart reads the new contents; renameSync would install
                // a new inode the running container never sees.
                fs.writeFileSync(filePath, content, 'utf8');
                fs.rmSync(stagedFilePath, { force: true });
              } else {
                fs.renameSync(stagedFilePath, filePath);
              }
            });

            if (ctx.isCertificateCreated) {
              previousConfig = configPaths.map((configPath) => [
                configPath,
                config.get(configPath),
              ]);
              configWasUpdated = true;

              config.set('platform.gateway.ssl.enabled', true);
              config.set('platform.gateway.ssl.provider', 'zerossl');
              config.set(
                'platform.gateway.ssl.providerConfigs.zerossl.id',
                ctx.certificate.id,
              );
              configFileRepository.write(configFile);
            }
          } catch (error) {
            let rollbackError;

            if (artifactInstallStarted) {
              stagedArtifacts.forEach(({ filePath, wasPresent, previousContent }) => {
                try {
                  if (wasPresent) {
                    fs.writeFileSync(filePath, previousContent, 'utf8');
                  } else {
                    fs.rmSync(filePath, { force: true });
                  }
                } catch (artifactRollbackError) {
                  rollbackError = rollbackError || artifactRollbackError;
                }
              });
            }

            if (configWasUpdated) {
              previousConfig.forEach(([configPath, value]) => config.set(configPath, value));

              try {
                configFileRepository.write(configFile);
              } catch (configRollbackError) {
                rollbackError = rollbackError || configRollbackError;
              }
            }

            if (rollbackError) {
              error.rollbackError = rollbackError;
            }

            throw error;
          } finally {
            try {
              fs.rmSync(stagingDir, { recursive: true, force: true });
            } catch {
              // A leftover staging directory is safe and must not mask the transaction result.
            }
          }

          // eslint-disable-next-line no-param-reassign
          task.output = artifacts.map(({ filePath }) => filePath).join(', ');
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
