import { CronJob } from 'cron';
import Certificate from '../ssl/zerossl/Certificate.js';

/**
 *
 * @param {getCertificate} getCertificate
 * @param {obtainZeroSSLCertificateTask} obtainZeroSSLCertificateTask
 * @param {DockerCompose} dockerCompose
 * @param {ConfigFileJsonRepository} configFileRepository
 * @param {ConfigFile} configFile
 * @param {writeConfigTemplates} writeConfigTemplates
 * @return {scheduleRenewZeroSslCertificate}
 */
export default function scheduleRenewZeroSslCertificateFactory(
  getCertificate,
  obtainZeroSSLCertificateTask,
  dockerCompose,
  configFileRepository,
  configFile,
  writeConfigTemplates,
) {
  /**
   * @typedef scheduleRenewZeroSslCertificate
   * @param {Config} config
   * @return {Promise<void>}
   */
  async function scheduleRenewZeroSslCertificate(config) {
    const certificate = await getCertificate(
      config.get('platform.gateway.ssl.providerConfigs.zerossl.apiKey', false),
      config.get('platform.gateway.ssl.providerConfigs.zerossl.id', false),
    );

    if (!certificate) {
      throw new Error('Invalid ZeroSSL certificate ID: certificate not found');
    }

    let expiresAt;
    if (certificate.isExpiredInDays(Certificate.EXPIRATION_LIMIT_DAYS)) {
      // Obtain new certificate right away
      expiresAt = new Date(Date.now() + 3000);

      // eslint-disable-next-line no-console
      console.log(`SSL certificate ${certificate.id} will expire in less than ${Certificate.EXPIRATION_LIMIT_DAYS} days at ${certificate.expires}. Schedule to obtain it NOW.`);
    } else {
      // Schedule a new check close to expiration period
      expiresAt = new Date(certificate.expires);
      expiresAt.setDate(expiresAt.getDate() - Certificate.EXPIRATION_LIMIT_DAYS);

      // eslint-disable-next-line no-console
      console.log(`SSL certificate ${certificate.id} will expire at ${certificate.expires}. Schedule to obtain at ${expiresAt}.`);
    }

    const job = new CronJob(expiresAt, async () => {
      const tasks = await obtainZeroSSLCertificateTask(config);

      await tasks.run({
        expirationDays: Certificate.EXPIRATION_LIMIT_DAYS,
        noRetry: true,
      });

      // Write config files
      configFileRepository.write(configFile);
      writeConfigTemplates(config);

      // TODO: We can use https://www.envoyproxy.io/docs/envoy/v1.30.1/start/quick-start/configuration-dynamic-filesystem.html#start-quick-start-dynamic-fs-dynamic-lds
      //  to dynamically update envoy configuration without restarting it

      // Restart Gateway to catch up new SSL certificates
      await dockerCompose.execCommand(config, 'gateway', 'kill -SIGHUP 1');

      return job.stop();
    }, async () => {
      // Schedule new cron task
      process.nextTick(() => scheduleRenewZeroSslCertificate(config));
    });

    job.start();
  }

  return scheduleRenewZeroSslCertificate;
}
