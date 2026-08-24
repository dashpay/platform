import ConfigIsNotPresentError from '../config/errors/ConfigIsNotPresentError.js';
import Certificate from '../ssl/zerossl/Certificate.js';
import { recordRenewalFailure } from './recordRenewalOutcome.js';
import scheduleRenewalJob from './scheduleRenewalJob.js';

/**
 *
 * @param {getCertificate} getCertificate
 * @param {obtainZeroSSLCertificateTask} obtainZeroSSLCertificateTask
 * @param {DockerCompose} dockerCompose
 * @param {ConfigFileJsonRepository} configFileRepository
 * @param {writeConfigTemplates} writeConfigTemplates
 * @param {HomeDir} homeDir
 * @return {scheduleRenewZeroSslCertificate}
 */
export default function scheduleRenewZeroSslCertificateFactory(
  getCertificate,
  obtainZeroSSLCertificateTask,
  dockerCompose,
  configFileRepository,
  writeConfigTemplates,
  homeDir,
) {
  /**
   * @typedef scheduleRenewZeroSslCertificate
   * @param {Config} config
   * @param {function(Config|null): Promise<boolean|void>} onConfigurationChanged
   * @return {Promise<void>}
   */
  async function scheduleRenewZeroSslCertificate(config, onConfigurationChanged) {
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
      console.error(`Failed to read configuration for ZeroSSL renewal, retrying in 1 hour: ${e.message}`);

      setTimeout(() => {
        scheduleRenewZeroSslCertificate(config, onConfigurationChanged);
      }, 60 * 60 * 1000);

      return;
    }

    if (!currentConfig.get('platform.gateway.ssl.enabled')
      || currentConfig.get('platform.gateway.ssl.provider') !== 'zerossl') {
      await onConfigurationChanged(currentConfig);

      return;
    }

    let certificate;
    try {
      certificate = await getCertificate(
        currentConfig.get('platform.gateway.ssl.providerConfigs.zerossl.apiKey', false),
        currentConfig.get('platform.gateway.ssl.providerConfigs.zerossl.id', false),
      );

      if (!certificate) {
        throw new Error('Invalid ZeroSSL certificate ID: certificate not found');
      }
    } catch (e) {
      // API failures must back off instead of terminating the helper's renewal chain.
      // eslint-disable-next-line no-console
      console.error(`Failed to read ZeroSSL certificate, retrying in 1 hour: ${e.message}`);

      // An account ZeroSSL refuses, or a certificate id it no longer knows,
      // stops renewal here permanently - no attempt is ever made, so nothing
      // downstream records anything. This is the state most of the expired
      // nodes on mainnet are in.
      recordRenewalFailure({
        homeDir,
        configName,
        provider: 'zerossl',
        error: e,
        apiKey: currentConfig.get('platform.gateway.ssl.providerConfigs.zerossl.apiKey', false),
      });

      setTimeout(() => {
        scheduleRenewZeroSslCertificate(config, onConfigurationChanged);
      }, 60 * 60 * 1000);

      return;
    }

    let expiresAt;
    // A failed validation checkpoints the remote certificate ID. Resume pending
    // certificates immediately; their expiry is not a meaningful retry time.
    // A certificate carrying no expiry date at all is handled here too: scheduling
    // from it would yield a date in the past, which cron rejects by throwing.
    if (certificate.status !== 'issued'
      || certificate.expires === null
      || certificate.isExpiredInDays(Certificate.EXPIRATION_LIMIT_DAYS)) {
      // Obtain new certificate right away
      expiresAt = new Date(Date.now() + 3000);

      // eslint-disable-next-line no-console
      console.log(`SSL certificate ${certificate.id} is not issued or will expire in less than ${Certificate.EXPIRATION_LIMIT_DAYS} days at ${certificate.expires}. Schedule to obtain it NOW.`);
    } else {
      // Schedule a new check close to expiration period
      expiresAt = new Date(certificate.expires);
      expiresAt.setDate(expiresAt.getDate() - Certificate.EXPIRATION_LIMIT_DAYS);

      // eslint-disable-next-line no-console
      console.log(`SSL certificate ${certificate.id} will expire at ${certificate.expires}. Schedule to obtain at ${expiresAt}.`);
    }

    scheduleRenewalJob({
      renewAt: expiresAt,
      currentConfig,
      provider: 'zerossl',
      providerName: 'ZeroSSL',
      expirationDays: Certificate.EXPIRATION_LIMIT_DAYS,
      obtainCertificateTask: obtainZeroSSLCertificateTask,
      configFileRepository,
      writeConfigTemplates,
      dockerCompose,
      homeDir,
      onConfigurationChanged,
      reschedule: (nextConfig) => scheduleRenewZeroSslCertificate(
        nextConfig,
        onConfigurationChanged,
      ),
    });
  }

  return scheduleRenewZeroSslCertificate;
}
