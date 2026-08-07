/**
 * Schedule renewal for the provider selected by the current configuration.
 *
 * @param {Config} config
 * @param {scheduleRenewZeroSslCertificate} scheduleRenewZeroSslCertificate
 * @param {scheduleRenewLetsEncryptCertificate} scheduleRenewLetsEncryptCertificate
 * @param {function(Config, function(Config|null): *): void} watchInactiveConfig
 * @return {Promise<void>}
 */
export default async function scheduleRenewCertificate(
  config,
  scheduleRenewZeroSslCertificate,
  scheduleRenewLetsEncryptCertificate,
  watchInactiveConfig,
) {
  const scheduleCurrentProvider = (currentConfig) => {
    if (currentConfig === null) {
      return watchInactiveConfig(config, scheduleCurrentProvider);
    }

    return scheduleRenewCertificate(
      currentConfig,
      scheduleRenewZeroSslCertificate,
      scheduleRenewLetsEncryptCertificate,
      watchInactiveConfig,
    );
  };
  const provider = config.get('platform.gateway.ssl.provider');

  if (config.get('platform.gateway.ssl.enabled') && provider === 'zerossl') {
    await scheduleRenewZeroSslCertificate(config, scheduleCurrentProvider);
    return;
  }

  if (config.get('platform.gateway.ssl.enabled') && provider === 'letsencrypt') {
    await scheduleRenewLetsEncryptCertificate(config, scheduleCurrentProvider);
    return;
  }

  watchInactiveConfig(config, scheduleCurrentProvider);
}
