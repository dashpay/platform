import { Listr } from 'listr2';
import fs from 'fs';
import path from 'path';
import os from 'os';

import { ERRORS } from '../../../../ssl/letsencrypt/validateLetsEncryptCertificateFactory.js';
import LegoCertificate from '../../../../ssl/letsencrypt/LegoCertificate.js';

const LEGO_IMAGE = 'goacme/lego:v4.31.0';

/**
 * @param {Docker} docker
 * @param {dockerPull} dockerPull
 * @param {StartedContainers} startedContainers
 * @param {HomeDir} homeDir
 * @param {validateLetsEncryptCertificate} validateLetsEncryptCertificate
 * @param {saveCertificateTask} saveCertificateTask
 * @param {ConfigFileJsonRepository} configFileRepository
 * @param {ConfigFile} configFile
 * @return {obtainLetsEncryptCertificateTask}
 */
export default function obtainLetsEncryptCertificateTaskFactory(
  docker,
  dockerPull,
  startedContainers,
  homeDir,
  validateLetsEncryptCertificate,
  saveCertificateTask,
  configFileRepository,
  configFile,
) {
  /**
   * @typedef {obtainLetsEncryptCertificateTask}
   * @param {Config} config
   * @return {Listr}
   */
  function obtainLetsEncryptCertificateTask(config) {
    return new Listr([
      {
        title: 'Check if certificate already exists and is valid',
        skip: (ctx) => ctx.force,
        task: async (ctx, task) => {
          const expirationDays = ctx.expirationDays || LegoCertificate.EXPIRATION_LIMIT_DAYS;
          const { error, data } = await validateLetsEncryptCertificate(config, expirationDays);

          Object.assign(ctx, data);

          // Ensure lego directory exists
          fs.mkdirSync(ctx.legoDir, { recursive: true });
          fs.mkdirSync(path.join(ctx.legoDir, 'certificates'), { recursive: true });

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
              throw new Error(`Certificate IP ${ctx.certificate.commonName} does not match external IP ${ctx.externalIp}.\n`
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

          // Determine if this is initial run or renewal
          const command = ctx.isRenewal ? 'renew' : 'run';

          // Build lego command arguments
          // --disable-cn is needed for IP address certificates
          const legoArgs = [
            '--server=https://acme-v02.api.letsencrypt.org/directory',
            '--email', ctx.email,
            '--accept-tos',
            '--http',
            '--http.port', ':80',
            '--domains', ctx.externalIp,
            '--disable-cn',
            '--path', '/data',
            command,
          ];

          // Add profile for initial run
          if (!ctx.isRenewal) {
            legoArgs.push('--profile', 'shortlived');
          } else {
            // For renewal, renew if within 30 days of expiry (default)
            legoArgs.push('--days', '30');
          }

          const containerName = 'dashmate-letsencrypt-lego';

          // Remove any existing container with the same name
          try {
            const existingContainer = await docker.getContainer(containerName);
            await existingContainer.remove({ force: true });
          } catch (e) {
            // Container doesn't exist, that's fine
            if (e.statusCode !== 404) {
              throw e;
            }
          }

          const container = await docker.createContainer({
            name: containerName,
            Image: LEGO_IMAGE,
            Cmd: legoArgs,
            User: `${uid}:${gid}`,
            ExposedPorts: { '80/tcp': {} },
            HostConfig: {
              AutoRemove: true,
              Binds: [`${ctx.legoDir}:/data`],
              PortBindings: { '80/tcp': [{ HostPort: '80' }] },
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

          // Verify certificate was created
          if (!fs.existsSync(ctx.legoCertPath)) {
            throw new Error('Certificate file was not created by lego');
          }

          // eslint-disable-next-line no-param-reassign
          task.output = 'Certificate obtained successfully';
        },
      },
      {
        title: 'Save certificate',
        skip: (ctx) => ctx.certificateValid,
        task: async (ctx) => {
          // Read certificate and key from lego output
          ctx.certificateFile = fs.readFileSync(ctx.legoCertPath, 'utf8');
          ctx.privateKeyFile = fs.readFileSync(ctx.legoKeyPath, 'utf8');

          // Update config
          config.set('platform.gateway.ssl.enabled', true);
          config.set('platform.gateway.ssl.provider', 'letsencrypt');

          // Save config file
          configFileRepository.write(configFile);

          // Save to gateway SSL directory
          return saveCertificateTask(config);
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
