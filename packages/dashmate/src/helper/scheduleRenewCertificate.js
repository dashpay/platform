/**
 * Schedule renewal for the provider selected by the current configuration.
 *
 * @param {Config} config
 * @param {scheduleRenewZeroSslCertificate} scheduleRenewZeroSslCertificate
 * @param {scheduleRenewLetsEncryptCertificate} scheduleRenewLetsEncryptCertificate
 * @param {function(Config, function(Config): Promise<boolean>): void} [watchInactiveConfig]
 * @return {Promise<boolean>} whether a scheduler or watcher was armed
 */
export default async function scheduleRenewCertificate(
  config,
  scheduleRenewZeroSslCertificate,
  scheduleRenewLetsEncryptCertificate,
  watchInactiveConfig = undefined,
) {
  const scheduleCurrentProvider = (currentConfig) => scheduleRenewCertificate(
    currentConfig,
    scheduleRenewZeroSslCertificate,
    scheduleRenewLetsEncryptCertificate,
    watchInactiveConfig,
  );
  const provider = config.get('platform.gateway.ssl.provider');

  if (config.get('platform.gateway.ssl.enabled') && provider === 'zerossl') {
    await scheduleRenewZeroSslCertificate(config, scheduleCurrentProvider);
    return true;
  }

  if (config.get('platform.gateway.ssl.enabled') && provider === 'letsencrypt') {
    await scheduleRenewLetsEncryptCertificate(config, scheduleCurrentProvider);
    return true;
  }

  if (watchInactiveConfig) {
    watchInactiveConfig(config, scheduleCurrentProvider);
    return true;
  }

  return false;
}
