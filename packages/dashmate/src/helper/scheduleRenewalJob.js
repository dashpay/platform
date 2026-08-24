import { CronJob } from 'cron';
import ServiceIsNotRunningError from '../docker/errors/ServiceIsNotRunningError.js';
import {
  clearRenewalRecord,
  recordGatewayReloadFailure,
  recordRenewalFailure,
  recordRenewalSuccess,
} from './recordRenewalOutcome.js';
import renewCertificate from './renewCertificate.js';
import watchCertificateConfig from './watchCertificateConfig.js';

export const RETRY_INTERVAL_MS = 60 * 60 * 1000;

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
 * @param {HomeDir} options.homeDir
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
  homeDir,
  onConfigurationChanged,
  reschedule,
}) {
  const configName = currentConfig.getName();
  let completion = 'retry';
  let nextConfig = currentConfig;
  let stopWatchingConfig = () => {};

  // Set when the renewal itself did not produce a certificate, and read after
  // the job is stopped. Recording from inside the catch below would put a write
  // ahead of job.stop(), which is the only thing that schedules the next
  // attempt - so a failure there would leave the helper running with nothing
  // scheduled and nothing watching the configuration.
  let renewalFailure = null;
  // Distinguished from the above because the certificate did renew. Counting a
  // signal that did not land as a failed renewal would tell an operator whose
  // certificate is minutes old that renewal has been failing for as long as
  // their previous certificate is old.
  let reloadFailure = null;
  // A failed signal still reaches the catch below, so the renewal's own verdict
  // cannot be read from whether one was raised.
  let isRenewed = false;

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

        // Renewal is no longer this provider's concern - SSL was turned off, or
        // the provider changed. Nothing will write here again, so anything left
        // behind would be reported forever against a node whose operator
        // deliberately stopped renewing.
        clearRenewalRecord({ homeDir, configName });

        completion = 'stop';
      } else {
        // The certificate exists from here on, whatever happens to the signal
        // below, so it is recorded before the signal is sent rather than after
        // the whole step succeeds.
        recordRenewalSuccess({ homeDir, configName, provider });

        isRenewed = true;

        // A signal is sufficient and nothing here needs to restart the
        // container. PID 1 in the gateway container is Envoy's hot-restarter,
        // not Envoy: its SIGHUP handler forks and re-execs Envoy with an
        // incremented restart epoch against the same envoy.yaml. The new
        // process parses that file from scratch and opens the certificate by
        // name, so the renewed certificate takes effect while the old process
        // drains. A container restart would achieve the same thing and cost an
        // outage.
        try {
          await dockerCompose.execCommand(renewal.config, 'gateway', 'kill -SIGHUP 1');
        } catch (e) {
          // A gateway that is down is not a certificate problem and is already
          // reported as a stopped service; the documented upgrade procedure
          // leaves it down on purpose. Anything else means the certificate is
          // installed and the gateway is still serving the previous one.
          reloadFailure = e instanceof ServiceIsNotRunningError ? null : e;

          throw e;
        }

        // eslint-disable-next-line no-console
        console.log(`${providerName} certificate renewed successfully`);

        completion = 'success';
      }
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error(`Failed to renew ${providerName} certificate: ${e.message}`);

      renewalFailure = e;
      completion = 'retry';
    }

    job.stop();

    // Only now that the next attempt is scheduled. Nothing below can throw -
    // recording swallows its own failures - but the ordering is what makes that
    // guarantee unnecessary rather than load-bearing.
    if (isRenewed) {
      // Nothing is recorded for a gateway that is simply down: that is not a
      // certificate problem, it is already reported as a stopped service, and
      // the renewal itself is already recorded as the success it was.
      if (reloadFailure !== null) {
        recordGatewayReloadFailure({ homeDir, configName });
      }
    } else if (renewalFailure !== null) {
      recordRenewalFailure({
        homeDir, configName, provider, error: renewalFailure,
      });
    }
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
