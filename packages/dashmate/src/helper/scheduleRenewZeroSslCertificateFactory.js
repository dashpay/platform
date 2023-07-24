const { CronJob } = require('cron');
const { EXPIRATION_LIMIT_DAYS } = require('../ssl/zerossl/Certificate');

/**
 *
 * @param {getCertificate} getCertificate
 * @param {obtainZeroSSLCertificateTask} obtainZeroSSLCertificateTask
 * @param {DockerCompose} dockerCompose
 * @return {scheduleRenewZeroSslCertificate}
 */
function scheduleRenewZeroSslCertificateFactory(
  getCertificate,
  obtainZeroSSLCertificateTask,
  dockerCompose,
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
    } else {
      // Schedule a new check close to expiration period
      expiresAt = new Date(certificate.expires);
      expiresAt.setDate(expiresAt.getDate() - EXPIRATION_LIMIT_DAYS);
    }

    const job = new CronJob(
      expiresAt, async () => {
        await obtainZeroSSLCertificateTask(config);

        // restart envoy
        const serviceInfo = await dockerCompose.inspectService(config, 'dapi_envoy');

        await dockerCompose.execCommand(config, serviceInfo.Id, 'kill -SIGHUP 1');

        return job.stop();
      }, async () => {
        // set up new cron
        process.nextTick(() => scheduleRenewZeroSslCertificate(config));
      },
    );

    job.start();
  }

  return scheduleRenewZeroSslCertificate;
}

module.exports = scheduleRenewZeroSslCertificateFactory;
