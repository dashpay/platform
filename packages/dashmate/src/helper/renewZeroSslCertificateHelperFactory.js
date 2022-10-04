const { CronJob } = require('cron');

/**
 *
 * @param {listCertificates} listCertificates
 * @param {renewZeroSSLCertificateTask} renewZeroSSLCertificateTask
 * @param {DockerCompose} dockerCompose
 * @return {renewZeroSslCertificateHelper}
 */
function renewZeroSslCertificateHelperFactory(
  listCertificates,
  renewZeroSSLCertificateTask,
  dockerCompose,
) {
  /**
   * @typedef renewZeroSslCertificateHelper
   * @param {Config} config
   * @return {Promise<void>}
   */
  async function renewZeroSslCertificateHelper(config) {
    const provider = config.get('platform.dapi.envoy.ssl.provider');
    if (provider !== 'zerossl') {
      return;
    }

    const certificatesResponse = await listCertificates(config.get('platform.dapi.envoy.ssl.providerConfigs.zerossl.apiKey'));

    const certificate = certificatesResponse.results.find((result) => result.common_name === config.get('externalIp'));

    if (!certificate) {
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
        process.nextTick(() => renewZeroSslCertificateHelper(config));
      },
    );

    job.start();
  }

  return renewZeroSslCertificateHelper;
}

module.exports = renewZeroSslCertificateHelperFactory;
