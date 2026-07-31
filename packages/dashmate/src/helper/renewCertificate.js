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

    const tasks = obtainCertificateTask(config);

    try {
      await tasks.run({
        expirationDays,
        noRetry: true,
      });
    } catch (e) {
      if (config.isChanged()) {
        // Failed renewal currently persists only a ZeroSSL certificate ID, which
        // is not consumed by templates. If partial renewal state ever affects a
        // template, render that state here before releasing the lock.
        configFileRepository.write(configFile);
      }

      throw e;
    }

    if (config.isChanged()) {
      configFileRepository.write(configFile);
      writeConfigTemplates(config);
    }

    return { config, renewed: true };
  } finally {
    configFileRepository.release();
  }
}
