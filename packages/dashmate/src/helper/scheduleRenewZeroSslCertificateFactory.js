const { CronJob } = require('cron');

/**
 *
 * @param {getCertificate} getCertificate
 * @param {renewZeroSSLCertificateTask} renewZeroSSLCertificateTask
 * @param {DockerCompose} dockerCompose
 * @return {scheduleRenewZeroSslCertificate}
 */
function scheduleRenewZeroSslCertificateFactory(
  getCertificate,
  renewZeroSSLCertificateTask,
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
      console.info('No ZeroSSL certificate found.');

      return;
    }

    let expiresAt = new Date(certificate.expires);
    expiresAt.setDay(expiresAt.getDay() - 7);

    if (expiresAt.getTime() < Date.now()) {
      expiresAt = new Date(Date.now() + 3000);
    }

    const job = new CronJob(
      expiresAt, async () => {
        await renewZeroSSLCertificateTask(config);
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
