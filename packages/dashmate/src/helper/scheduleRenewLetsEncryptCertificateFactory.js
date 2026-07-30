import { CronJob } from 'cron';
import path from 'path';

import LegoCertificate from '../ssl/letsencrypt/LegoCertificate.js';

/**
 * @param {obtainLetsEncryptCertificateTask} obtainLetsEncryptCertificateTask
 * @param {DockerCompose} dockerCompose
 * @param {ConfigFileJsonRepository} configFileRepository
 * @param {ConfigFile} configFile
 * @param {writeConfigTemplates} writeConfigTemplates
 * @param {HomeDir} homeDir
 * @return {scheduleRenewLetsEncryptCertificate}
 */
export default function scheduleRenewLetsEncryptCertificateFactory(
  obtainLetsEncryptCertificateTask,
  dockerCompose,
  configFileRepository,
  configFile,
  writeConfigTemplates,
  homeDir,
) {
  /**
   * @typedef scheduleRenewLetsEncryptCertificate
   * @param {Config} config
   * @return {Promise<void>}
   */
  async function scheduleRenewLetsEncryptCertificate(config) {
    const externalIp = config.get('externalIp');
    const legoDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'lego');
    const certPath = path.join(legoDir, 'certificates', `${externalIp}.crt`);

    let certificate;
    try {
      certificate = LegoCertificate.fromFile(certPath);
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error(`Failed to read Let's Encrypt certificate from ${certPath}: ${e.message}`);
      // Schedule a check in 1 hour to see if certificate appears
      const retryAt = new Date(Date.now() + 60 * 60 * 1000);

      const retryJob = new CronJob(retryAt, async () => {
        retryJob.stop();
        process.nextTick(() => scheduleRenewLetsEncryptCertificate(config));
      });

      retryJob.start();
      return;
    }

    let renewAt;
    if (certificate.isExpiredInDays(LegoCertificate.EXPIRATION_LIMIT_DAYS)) {
      // Obtain new certificate right away
      renewAt = new Date(Date.now() + 3000);

      // eslint-disable-next-line no-console
      console.log(`Let's Encrypt certificate will expire in less than ${LegoCertificate.EXPIRATION_LIMIT_DAYS} days at ${certificate.expires}. Schedule to obtain it NOW.`);
    } else {
      // Schedule a new check close to expiration period
      renewAt = new Date(certificate.expires);
      renewAt.setDate(renewAt.getDate() - LegoCertificate.EXPIRATION_LIMIT_DAYS);

      // eslint-disable-next-line no-console
      console.log(`Let's Encrypt certificate will expire at ${certificate.expires}. Schedule to obtain at ${renewAt}.`);
    }

    let renewalSucceeded = false;

    const job = new CronJob(renewAt, async () => {
      try {
        const tasks = obtainLetsEncryptCertificateTask(config);

        await tasks.run({
          expirationDays: LegoCertificate.EXPIRATION_LIMIT_DAYS,
          noRetry: true,
        });

        // This process has held its copy of the config file since it started,
        // possibly for months, so saving that copy would revert everything
        // changed on the node since. Re-apply just what the renewal produced
        // onto the current state instead.
        const renewedOptions = config.get('platform.gateway.ssl');

        configFileRepository.update((freshConfigFile) => {
          freshConfigFile.getConfig(config.getName())
            .set('platform.gateway.ssl', renewedOptions);
        }, {
          onSaved: (savedConfigFile) => writeConfigTemplates(
            savedConfigFile.getConfig(config.getName()),
          ),
        });

        // Restart Gateway to catch up new SSL certificates
        await dockerCompose.execCommand(config, 'gateway', 'kill -SIGHUP 1');

        // eslint-disable-next-line no-console
        console.log("Let's Encrypt certificate renewed successfully");

        renewalSucceeded = true;
      } catch (e) {
        // eslint-disable-next-line no-console
        console.error(`Failed to renew Let's Encrypt certificate: ${e.message}`);

        renewalSucceeded = false;
      }

      job.stop();
    }, async () => {
      // Schedule new cron task after completion
      if (renewalSucceeded) {
        // Success: reschedule immediately to read new cert expiry
        process.nextTick(() => scheduleRenewLetsEncryptCertificate(config));
      } else {
        // Failure: wait 1 hour before retrying to avoid tight loop
        // eslint-disable-next-line no-console
        console.log("Scheduling Let's Encrypt renewal retry in 1 hour");

        setTimeout(() => {
          scheduleRenewLetsEncryptCertificate(config);
        }, 60 * 60 * 1000);
      }
    });

    job.start();
  }

  return scheduleRenewLetsEncryptCertificate;
}
