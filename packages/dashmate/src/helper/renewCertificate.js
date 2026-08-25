/**
 * Obtain and persist a renewed certificate against the latest configuration.
 *
 * The lock intentionally spans certificate issuance and HTTP validation. This can
 * make foreground writers hit their acquire timeout, but prevents a provider switch
 * or SSL disable from being overwritten by replaying renewal fields after the fact.
 *
 * @param {Object} options
 * @param {string} options.configName
 * @param {string} options.provider
 * @param {number} options.expirationDays
 * @param {function(Config): Listr} options.obtainCertificateTask
 * @param {number|null} [options.generation] - the scheduling chain's fence, so
 *   an install performed inside a renewal does not lock that renewal out of
 *   recording the success it just achieved
 * @param {ConfigFileJsonRepository} options.configFileRepository
 * @param {writeConfigTemplates} options.writeConfigTemplates
 * @return {Promise<{config: Config, renewed: boolean}>}
 */
export default async function renewCertificate({
  configName,
  provider,
  expirationDays,
  generation = null,
  obtainCertificateTask,
  configFileRepository,
  writeConfigTemplates,
}) {
  configFileRepository.acquire();

  try {
    const { configFile } = configFileRepository.readAndMigrate(
      {},
      (migratedConfigs) => migratedConfigs.forEach(writeConfigTemplates),
    );
    const config = configFile.getConfig(configName);

    if (!config.get('platform.gateway.ssl.enabled')
      || config.get('platform.gateway.ssl.provider') !== provider) {
      return { config, renewed: false };
    }

    const tasks = obtainCertificateTask(config, {
      onCertificateCreated: () => configFileRepository.write(configFile),
    });

    try {
      await tasks.run({
        expirationDays,
        noRetry: true,
        renewalGeneration: generation,
      });
    } catch (e) {
      if (config.isChanged()) {
        // Preserve recoverable provider state before propagating a failed renewal.
        configFileRepository.write(configFile);
      }

      throw e;
    }

    if (config.isChanged()) {
      // Issuance can take minutes, which is long enough for this lease to be
      // lost and another command to save and render newer state. Rendering from
      // this configuration would overwrite that, and the save's own check comes
      // too late to prevent it.
      if (!configFileRepository.isExclusive()) {
        throw new Error('Lost the configuration lock while renewing the certificate,'
          + ' so the gateway service files were not written. The certificate was'
          + ' obtained; re-run renewal once no other command is changing configuration.');
      }

      // JSON is authoritative. If rendering fails or the helper is killed next,
      // an explicit config render repairs the stale gateway service files.
      configFileRepository.write(configFile);

      writeConfigTemplates(config);
    }

    return { config, renewed: true };
  } finally {
    configFileRepository.release();
  }
}
