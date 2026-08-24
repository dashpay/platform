import { CronJob } from 'cron';
import path from 'path';

import ConfigIsNotPresentError from '../config/errors/ConfigIsNotPresentError.js';
import LegoCertificate from '../ssl/letsencrypt/LegoCertificate.js';
import isCertificatePairInstalled from '../ssl/letsencrypt/isCertificatePairInstalled.js';
import { recordRenewalFailure } from './recordRenewalOutcome.js';
import scheduleRenewalJob from './scheduleRenewalJob.js';

/**
 * @param {obtainLetsEncryptCertificateTask} obtainLetsEncryptCertificateTask
 * @param {DockerCompose} dockerCompose
 * @param {ConfigFileJsonRepository} configFileRepository
 * @param {writeConfigTemplates} writeConfigTemplates
 * @param {HomeDir} homeDir
 * @return {scheduleRenewLetsEncryptCertificate}
 */
export default function scheduleRenewLetsEncryptCertificateFactory(
  obtainLetsEncryptCertificateTask,
  dockerCompose,
  configFileRepository,
  writeConfigTemplates,
  homeDir,
) {
  /**
   * @typedef scheduleRenewLetsEncryptCertificate
   * @param {Config} config
   * @param {function(Config|null): Promise<boolean|void>} onConfigurationChanged
   * @return {Promise<void>}
   */
  async function scheduleRenewLetsEncryptCertificate(config, onConfigurationChanged) {
    const configName = config.getName();
    let currentConfig;

    try {
      currentConfig = configFileRepository.read().getConfig(configName);
    } catch (e) {
      if (e instanceof ConfigIsNotPresentError) {
        await onConfigurationChanged(null);
        return;
      }

      // A transient read failure must not terminate the helper's only renewal chain.
      // eslint-disable-next-line no-console
      console.error(`Failed to read configuration for Let's Encrypt renewal, retrying in 1 hour: ${e.message}`);

      setTimeout(() => {
        scheduleRenewLetsEncryptCertificate(config, onConfigurationChanged);
      }, 60 * 60 * 1000);

      return;
    }

    if (!currentConfig.get('platform.gateway.ssl.enabled')
      || currentConfig.get('platform.gateway.ssl.provider') !== 'letsencrypt') {
      await onConfigurationChanged(currentConfig);

      return;
    }

    const externalIp = currentConfig.get('externalIp');
    const legoDir = homeDir.joinPath(configName, 'platform', 'gateway', 'lego');
    const certPath = path.join(legoDir, 'certificates', `${externalIp}.crt`);
    const keyPath = path.join(legoDir, 'certificates', `${externalIp}.key`);
    const gatewayDir = homeDir.joinPath(configName, 'platform', 'gateway', 'ssl');
    const gatewayCertPath = path.join(gatewayDir, 'bundle.crt');
    const gatewayKeyPath = path.join(gatewayDir, 'private.key');

    let certificate;
    try {
      certificate = LegoCertificate.fromFile(certPath);
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error(`Failed to read Let's Encrypt certificate from ${certPath}: ${e.message}`);

      // Renewal never reaches an attempt from here - it re-checks hourly for a
      // file that will not appear on its own - so without recording it, a node
      // in this state renews nothing and says nothing about why.
      //
      // The error itself decides the cause. This read throws for an absent
      // file, for a permission denial and for a corrupt certificate alike, and
      // only the first of those is repaired by obtaining a new one.
      recordRenewalFailure({
        homeDir,
        configName,
        provider: 'letsencrypt',
        error: e,
      });

      // Schedule a check in 1 hour to see if certificate appears
      const retryAt = new Date(Date.now() + 60 * 60 * 1000);

      const retryJob = new CronJob(retryAt, async () => {
        retryJob.stop();
        process.nextTick(
          () => scheduleRenewLetsEncryptCertificate(config, onConfigurationChanged),
        );
      });

      retryJob.start();
      return;
    }

    const isInstalled = isCertificatePairInstalled(
      certPath,
      keyPath,
      gatewayCertPath,
      gatewayKeyPath,
    );
    let renewAt;
    if (!isInstalled
      || certificate.isExpiredInDays(LegoCertificate.EXPIRATION_LIMIT_DAYS)) {
      // Obtain new certificate right away
      renewAt = new Date(Date.now() + 3000);

      // eslint-disable-next-line no-console
      console.log(`Let's Encrypt certificate is not installed or will expire in less than ${LegoCertificate.EXPIRATION_LIMIT_DAYS} days at ${certificate.expires}. Schedule to obtain it NOW.`);
    } else {
      // Schedule a new check close to expiration period
      renewAt = new Date(certificate.expires);
      renewAt.setDate(renewAt.getDate() - LegoCertificate.EXPIRATION_LIMIT_DAYS);

      // eslint-disable-next-line no-console
      console.log(`Let's Encrypt certificate will expire at ${certificate.expires}. Schedule to obtain at ${renewAt}.`);
    }

    scheduleRenewalJob({
      renewAt,
      currentConfig,
      provider: 'letsencrypt',
      providerName: "Let's Encrypt",
      expirationDays: LegoCertificate.EXPIRATION_LIMIT_DAYS,
      obtainCertificateTask: obtainLetsEncryptCertificateTask,
      configFileRepository,
      writeConfigTemplates,
      dockerCompose,
      homeDir,
      onConfigurationChanged,
      reschedule: (nextConfig) => scheduleRenewLetsEncryptCertificate(
        nextConfig,
        onConfigurationChanged,
      ),
    });
  }

  return scheduleRenewLetsEncryptCertificate;
}
