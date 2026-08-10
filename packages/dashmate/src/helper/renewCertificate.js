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
 * @param {ConfigFileJsonRepository} options.configFileRepository
 * @param {writeConfigTemplates} options.writeConfigTemplates
 * @return {Promise<{config: Config, renewed: boolean}>}
 */
export default async function renewCertificate({
  configName,
  provider,
  expirationDays,
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

      // Rendering before the save keeps a failure recoverable: nothing is
      // committed, so the next renewal attempt redoes both. Saving first would
      // leave the gateway's generated files behind a configuration that already
      // claims the new certificate, with nothing to trigger a re-render.
      writeConfigTemplates(config);

      configFileRepository.write(configFile);
    }

    return { config, renewed: true };
  } finally {
    configFileRepository.release();
  }
}
