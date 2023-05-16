const { CronJob } = require('cron');

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
      config.get('platform.dapi.envoy.ssl.providerConfigs.zerossl.apiKey'),
      config.get('platform.dapi.envoy.ssl.providerConfigs.zerossl.id'),
    );

    if (!certificate) {
      // eslint-disable-next-line no-console
      console.log('No ZeroSSL certificate found.');

      return;
    }

    let expiresAt = new Date(certificate.expires);
    expiresAt.setDate(expiresAt.getDate() - 7);

    if (expiresAt.getTime() < Date.now()) {
      expiresAt = new Date(Date.now() + 3000);
    }

    const job = new CronJob(
      expiresAt, async () => {
        await obtainZeroSSLCertificateTask(config);
        // restart envoy
        const serviceInfo = await dockerCompose.inspectService(config.toEnvs(), 'dapi_envoy');

        await dockerCompose.execCommand(config.toEnvs(), serviceInfo.Id, 'kill -SIGHUP 1');

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
