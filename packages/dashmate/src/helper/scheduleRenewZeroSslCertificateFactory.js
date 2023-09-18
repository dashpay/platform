const { CronJob } = require('cron');
const { EXPIRATION_LIMIT_DAYS } = require('../ssl/zerossl/Certificate');

/**
 *
 * @param {getCertificate} getCertificate
 * @param {obtainZeroSSLCertificateTask} obtainZeroSSLCertificateTask
 * @param {DockerCompose} dockerCompose
 * @param {ConfigFileJsonRepository} configFileRepository
 * @param {ConfigFile} configFile
 * @param {renderServiceTemplates} renderServiceTemplates
 * @param {writeServiceConfigs} writeServiceConfigs
 * @return {scheduleRenewZeroSslCertificate}
 */
function scheduleRenewZeroSslCertificateFactory(
  getCertificate,
  obtainZeroSSLCertificateTask,
  dockerCompose,
  configFileRepository,
  configFile,
  renderServiceTemplates,
  writeServiceConfigs,
) {
  /**
   * @typedef scheduleRenewZeroSslCertificate
   * @param {Config} config
   * @return {Promise<void>}
   */
  async function scheduleRenewZeroSslCertificate(config) {
    const certificate = await getCertificate(
      config.get('platform.dapi.envoy.ssl.providerConfigs.zerossl.apiKey', false),
      config.get('platform.dapi.envoy.ssl.providerConfigs.zerossl.id', false),
    );

    if (!certificate) {
      throw new Error('Invalid ZeroSSL certificate ID: certificate not found');
    }

    let expiresAt;
    if (certificate.isExpiredInDays(EXPIRATION_LIMIT_DAYS)) {
      // Obtain new certificate right away
      expiresAt = new Date(Date.now() + 3000);

      // eslint-disable-next-line no-console
      console.log(`SSL certificate ${certificate.id} will expire in less than ${EXPIRATION_LIMIT_DAYS} days at ${certificate.expires}. Schedule to obtain it NOW.`);
    } else {
      // Schedule a new check close to expiration period
      expiresAt = new Date(certificate.expires);
      expiresAt.setDate(expiresAt.getDate() - EXPIRATION_LIMIT_DAYS);

      // eslint-disable-next-line no-console
      console.log(`SSL certificate ${certificate.id} will expire at ${certificate.expires}. Schedule to obtain at ${expiresAt}.`);
    }

    const job = new CronJob(
      expiresAt, async () => {
        const tasks = await obtainZeroSSLCertificateTask(config);

        await tasks.run({
          expirationDays: EXPIRATION_LIMIT_DAYS,
        });

        // Write config files
        configFileRepository.write(configFile);

        const serviceConfigs = renderServiceTemplates(config);
        writeServiceConfigs(config.getName(), serviceConfigs);

        // Restart Envoy to catch up new SSL certificates
        await dockerCompose.execCommand(config, 'dapi_envoy', 'kill -SIGHUP 1');

        return job.stop();
      }, async () => {
        // Schedule new cron task
        process.nextTick(() => scheduleRenewZeroSslCertificate(config));
      },
    );

    job.start();
  }

  return scheduleRenewZeroSslCertificate;
}

module.exports = scheduleRenewZeroSslCertificateFactory;
