const { CronJob } = require('cron');
const generateEnvs = require('../util/generateEnvs');

/**
 *
 * @param {getCertificate} getCertificate
 * @param {obtainZeroSSLCertificateTask} obtainZeroSSLCertificateTask
 * @param {DockerCompose} dockerCompose
 * @param {ConfigFile} configFile
 * @return {scheduleRenewZeroSslCertificate}
 */
function scheduleRenewZeroSslCertificateFactory(
  getCertificate,
  obtainZeroSSLCertificateTask,
  dockerCompose,
  configFile,
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
        const serviceInfo = await dockerCompose.inspectService(generateEnvs(configFile, config), 'dapi_envoy');

        await dockerCompose.execCommand(generateEnvs(configFile, config), serviceInfo.Id, 'kill -SIGHUP 1');

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
