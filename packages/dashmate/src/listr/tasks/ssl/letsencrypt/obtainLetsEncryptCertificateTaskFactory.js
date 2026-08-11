import { Listr } from 'listr2';
import fs from 'fs';
import path from 'path';
import os from 'os';

import { ERRORS } from '../../../../ssl/letsencrypt/validateLetsEncryptCertificateFactory.js';
import LegoCertificate from '../../../../ssl/letsencrypt/LegoCertificate.js';

const LEGO_IMAGE = 'goacme/lego:v4.31.0';

const LEGO_CA_CERTIFICATE_MOUNT_PATH = '/acme-ca.pem';

/**
 * @param {Docker} docker
 * @param {dockerPull} dockerPull
 * @param {StartedContainers} startedContainers
 * @param {HomeDir} homeDir
 * @param {validateLetsEncryptCertificate} validateLetsEncryptCertificate
 * @param {saveCertificateTask} saveCertificateTask
 * @param {string|null} legoCaCertificatePath - CA that signed the ACME
 *   directory's own certificate, for a directory that is not publicly trusted
 * @param {Object} legoContainerOptions - Docker create-container overrides for
 *   the lego container, so it can be reached over a network of the caller's
 *   choosing rather than the host's port 80
 * @return {obtainLetsEncryptCertificateTask}
 */
export default function obtainLetsEncryptCertificateTaskFactory(
  docker,
  dockerPull,
  startedContainers,
  homeDir,
  validateLetsEncryptCertificate,
  saveCertificateTask,
  legoCaCertificatePath,
  legoContainerOptions,
) {
  /**
   * @typedef {obtainLetsEncryptCertificateTask}
   * @param {Config} config
   * @return {Listr}
   */
  function obtainLetsEncryptCertificateTask(config) {
    return new Listr([
      {
        title: 'Initialize configuration',
        task: async (ctx) => {
          // Always load config values (needed even when --force is used)
          ctx.email = config.get('platform.gateway.ssl.providerConfigs.letsencrypt.email');
          ctx.acmeDirectoryUrl = config.get('platform.gateway.ssl.providerConfigs.letsencrypt.acmeDirectoryUrl');
          ctx.externalIp = config.get('externalIp');
          ctx.legoDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'lego');
          ctx.sslConfigDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl');
          ctx.configurationUpdateRequired = !config.get('platform.gateway.ssl.enabled')
            || config.get('platform.gateway.ssl.provider') !== 'letsencrypt';

          if (!ctx.email) {
            throw new Error("Let's Encrypt email is not set. Please set it in the config file");
          }

          if (!ctx.externalIp) {
            throw new Error('External IP is not set. Please set it in the config file');
          }

          // Ensure lego directories exist
          fs.mkdirSync(ctx.legoDir, { recursive: true });
          fs.mkdirSync(path.join(ctx.legoDir, 'certificates'), { recursive: true });
          fs.mkdirSync(path.join(ctx.legoDir, 'accounts'), { recursive: true });

          // Set paths
          ctx.legoCertPath = path.join(ctx.legoDir, 'certificates', `${ctx.externalIp}.crt`);
          ctx.legoKeyPath = path.join(ctx.legoDir, 'certificates', `${ctx.externalIp}.key`);

          // When force is used, skip validation and obtain new certificate
          if (ctx.force) {
            ctx.certificateValid = false;
            ctx.isRenewal = false;
          }
        },
      },
      {
        title: 'Check if certificate already exists and is valid',
        skip: (ctx) => ctx.force,
        task: async (ctx, task) => {
          const expirationDays = ctx.expirationDays ?? LegoCertificate.EXPIRATION_LIMIT_DAYS;
          const { error, data } = await validateLetsEncryptCertificate(config, expirationDays);

          // Merge validation data (but don't overwrite already-set values)
          Object.keys(data).forEach((key) => {
            if (ctx[key] === undefined) {
              ctx[key] = data[key];
            }
          });

          switch (error) {
            case undefined:
              ctx.certificateValid = true;
              // eslint-disable-next-line no-param-reassign
              task.output = `Certificate is valid and expires at ${ctx.certificate.expires}`;
              break;
            case ERRORS.EMAIL_IS_NOT_SET:
              throw new Error('Let\'s Encrypt email is not set. Please set it in the config file');
            case ERRORS.EXTERNAL_IP_IS_NOT_SET:
              throw new Error('External IP is not set. Please set it in the config file');
            case ERRORS.CERTIFICATE_NOT_FOUND:
              // eslint-disable-next-line no-param-reassign
              task.output = 'Certificate not found, obtaining a new one';
              ctx.certificateValid = false;
              ctx.isRenewal = false;
              break;
            case ERRORS.PRIVATE_KEY_NOT_FOUND:
              // eslint-disable-next-line no-param-reassign
              task.output = 'Private key not found, obtaining a new certificate';
              ctx.certificateValid = false;
              ctx.isRenewal = false;
              break;
            case ERRORS.CERTIFICATE_EXPIRES_SOON:
              // eslint-disable-next-line no-param-reassign
              task.output = `Certificate expires soon at ${ctx.certificate.expires}, renewing`;
              ctx.certificateValid = false;
              ctx.isRenewal = true;
              break;
            case ERRORS.CERTIFICATE_IP_MISMATCH:
              throw new Error(`Certificate does not match external IP ${ctx.externalIp}.\n`
                + 'Please change the external IP in config or use --force to obtain a new certificate.');
            case ERRORS.CERTIFICATE_NOT_VALID:
              // eslint-disable-next-line no-param-reassign
              task.output = 'Certificate is not valid, obtaining a new one';
              ctx.certificateValid = false;
              ctx.isRenewal = false;
              break;
            default:
              throw new Error(`Unknown error: ${error}`);
          }
        },
      },
      {
        title: `Pull lego Docker image (${LEGO_IMAGE})`,
        skip: (ctx) => ctx.certificateValid,
        task: async () => {
          await dockerPull(LEGO_IMAGE);
        },
      },
      {
        title: 'Obtain certificate using lego',
        skip: (ctx) => ctx.certificateValid,
        task: async (ctx, task) => {
          const { uid, gid } = os.userInfo();

          let acmeDirectoryUrl;
          try {
            acmeDirectoryUrl = new URL(ctx.acmeDirectoryUrl);
          } catch {
            throw new Error('ACME directory URL must use HTTPS');
          }

          if (acmeDirectoryUrl.protocol !== 'https:') {
            throw new Error('ACME directory URL must use HTTPS');
          }

          // Determine if this is initial run or renewal
          const command = ctx.isRenewal ? 'renew' : 'run';

          // Build lego command arguments
          // --disable-cn is needed for IP address certificates
          // --key-type rsa2048 is needed because node-forge doesn't support ECDSA
          const legoArgs = [
            `--server=${acmeDirectoryUrl.toString()}`,
            '--email', ctx.email,
            '--accept-tos',
            '--http',
            '--http.port', ':80',
            '--domains', ctx.externalIp,
            '--disable-cn',
            '--key-type', 'rsa2048',
            '--path', '/data',
            command,
          ];

          // shortlived profile is required for IP address certificates
          legoArgs.push('--profile', 'shortlived');

          if (ctx.isRenewal) {
            legoArgs.push('--days', '30');
          }

          const containerName = 'dashmate-letsencrypt-lego';

          // Remove any existing container with the same name
          try {
            const existingContainer = await docker.getContainer(containerName);
            await existingContainer.remove({ force: true });

            try {
              await existingContainer.wait();
            } catch (waitError) {
              // Skip error if container is already removed
              if (waitError.statusCode !== 404) {
                throw waitError;
              }
            }
          } catch (e) {
            // Container doesn't exist, that's fine
            if (e.statusCode !== 404) {
              throw e;
            }
          }

          const binds = [`${ctx.legoDir}:/data`];
          const env = [];

          // An ACME directory that is not publicly trusted - a staging or local
          // server - presents a certificate lego rejects unless told which CA
          // signed it.
          if (legoCaCertificatePath) {
            binds.push(`${legoCaCertificatePath}:${LEGO_CA_CERTIFICATE_MOUNT_PATH}:ro`);
            env.push(`LEGO_CA_CERTIFICATES=${LEGO_CA_CERTIFICATE_MOUNT_PATH}`);
          }

          const container = await docker.createContainer({
            name: containerName,
            Image: LEGO_IMAGE,
            Cmd: legoArgs,
            Env: env,
            User: `${uid}:${gid}`,
            ExposedPorts: { '80/tcp': {} },
            ...legoContainerOptions,
            HostConfig: {
              AutoRemove: true,
              Binds: binds,
              PortBindings: { '80/tcp': [{ HostPort: '80' }] },
              ...legoContainerOptions.HostConfig,
            },
          });

          startedContainers.addContainer(containerName);

          // eslint-disable-next-line no-param-reassign
          task.output = `Running lego ${command}...`;

          await container.start();

          // Wait for container to finish
          const result = await container.wait();

          if (result.StatusCode !== 0) {
            // Try to get logs for error message
            let errorMessage = `Lego exited with code ${result.StatusCode}`;
            try {
              const logs = await container.logs({
                stdout: true,
                stderr: true,
              });
              errorMessage += `\n${logs.toString()}`;
            } catch (e) {
              // Container may have been auto-removed
            }

            throw new Error(`Failed to obtain Let's Encrypt certificate: ${errorMessage}\n`
              + `Please ensure port 80 on your public IP address ${ctx.externalIp} is open\n`
              + 'for incoming HTTP connections.');
          }

          // Verify certificate and key were created
          if (!fs.existsSync(ctx.legoCertPath)) {
            throw new Error('Certificate file was not created by lego');
          }

          if (!fs.existsSync(ctx.legoKeyPath)) {
            throw new Error('Private key file was not created by lego');
          }

          ctx.configurationUpdateRequired = true;

          // eslint-disable-next-line no-param-reassign
          task.output = 'Certificate obtained successfully';
        },
      },
      {
        title: 'Save certificate',
        skip: (ctx) => ctx.certificateValid && ctx.isCertificatePairInstalled,
        task: async (ctx) => {
          // Read certificate and key from lego output
          ctx.certificateFile = fs.readFileSync(ctx.legoCertPath, 'utf8');
          ctx.privateKeyFile = fs.readFileSync(ctx.legoKeyPath, 'utf8');
          ctx.configurationUpdateRequired = true;

          // Save to gateway SSL directory
          return saveCertificateTask(config);
        },
      },
      {
        title: 'Update configuration',
        enabled: (ctx) => ctx.configurationUpdateRequired,
        task: async () => {
          config.set('platform.gateway.ssl.enabled', true);
          config.set('platform.gateway.ssl.provider', 'letsencrypt');
        },
      },
    ], {
      rendererOptions: {
        showErrorMessage: true,
      },
    });
  }

  return obtainLetsEncryptCertificateTask;
}
