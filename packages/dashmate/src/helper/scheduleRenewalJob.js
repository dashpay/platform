import { CronJob } from 'cron';
import renewCertificate from './renewCertificate.js';
import watchCertificateConfig from './watchCertificateConfig.js';

const RETRY_INTERVAL_MS = 60 * 60 * 1000;

/**
 * Run a scheduled renewal while allowing a configuration change to supersede it.
 *
 * @param {Object} options
 * @param {Date} options.renewAt
 * @param {Config} options.currentConfig
 * @param {string} options.provider
 * @param {string} options.providerName
 * @param {number} options.expirationDays
 * @param {function(Config): Listr} options.obtainCertificateTask
 * @param {ConfigFileJsonRepository} options.configFileRepository
 * @param {writeConfigTemplates} options.writeConfigTemplates
 * @param {DockerCompose} options.dockerCompose
 * @param {function(Config): Promise<boolean>} options.onConfigurationChanged
 * @param {function(Config): Promise<void>} options.reschedule
 */
export default function scheduleRenewalJob({
  renewAt,
  currentConfig,
  provider,
  providerName,
  expirationDays,
  obtainCertificateTask,
  configFileRepository,
  writeConfigTemplates,
  dockerCompose,
  onConfigurationChanged,
  reschedule,
}) {
  const configName = currentConfig.getName();
  let completion = 'retry';
  let nextConfig = currentConfig;
  let stopWatchingConfig = () => {};

  const job = new CronJob(renewAt, async () => {
    stopWatchingConfig();

    try {
      const renewal = await renewCertificate({
        configName,
        provider,
        expirationDays,
        obtainCertificateTask,
        configFileRepository,
        writeConfigTemplates,
      });

      nextConfig = renewal.config;

      if (!renewal.renewed) {
        await onConfigurationChanged(renewal.config);

        completion = 'stop';
      } else {
        // A signal is sufficient and nothing here needs to restart the
        // container. PID 1 in the gateway container is Envoy's hot-restarter,
        // not Envoy: its SIGHUP handler forks and re-execs Envoy with an
        // incremented restart epoch against the same envoy.yaml. The new
        // process parses that file from scratch and opens the certificate by
        // name, so the renewed certificate takes effect while the old process
        // drains. A container restart would achieve the same thing and cost an
        // outage.
        await dockerCompose.execCommand(renewal.config, 'gateway', 'kill -SIGHUP 1');

        // eslint-disable-next-line no-console
        console.log(`${providerName} certificate renewed successfully`);

        completion = 'success';
      }
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error(`Failed to renew ${providerName} certificate: ${e.message}`);

      completion = 'retry';
    }

    job.stop();
  }, () => {
    if (completion === 'stop') {
      return;
    }

    if (completion === 'success') {
      process.nextTick(() => reschedule(nextConfig));
      return;
    }

    // eslint-disable-next-line no-console
    console.log(`Scheduling ${providerName} renewal retry in 1 hour`);

    setTimeout(() => reschedule(nextConfig), RETRY_INTERVAL_MS);
  });

  stopWatchingConfig = watchCertificateConfig(
    currentConfig,
    provider,
    configFileRepository,
    async (changedConfig) => {
      completion = 'stop';
      job.stop();

      if (changedConfig) {
        return onConfigurationChanged(changedConfig);
      }

      return false;
    },
    (e) => {
      // eslint-disable-next-line no-console
      console.error(`Failed to check configuration for ${providerName} renewal: ${e.message}`);
    },
  );

  job.start();
}
