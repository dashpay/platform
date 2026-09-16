import ConfigIsNotPresentError from '../config/errors/ConfigIsNotPresentError.js';

export const CONFIG_REFRESH_INTERVAL_MS = 60 * 1000;

/**
 * @param {Config} config
 * @param {string|null} provider
 * @return {string}
 */
function getRenewalConfigurationFingerprint(config, provider) {
  const values = [
    config.get('externalIp'),
    config.get('platform.gateway.ssl.enabled'),
    config.get('platform.gateway.ssl.provider'),
  ];

  if (provider === 'zerossl') {
    values.push(
      config.get('platform.gateway.ssl.providerConfigs.zerossl.apiKey'),
      config.get('platform.gateway.ssl.providerConfigs.zerossl.id'),
    );
  } else if (provider === 'letsencrypt') {
    values.push(config.get('platform.gateway.ssl.providerConfigs.letsencrypt.email'));
  }

  return JSON.stringify(values);
}

/**
 * Watch the local config for changes that make a scheduled renewal obsolete.
 * Without this poll, a provider switch made after a distant renewal was scheduled
 * would not be handed to the new provider until the old job eventually fired.
 *
 * @param {Config} scheduledConfig
 * @param {string|null} provider
 * @param {ConfigFileJsonRepository} configFileRepository
 * @param {function(Config|null): Promise<boolean|void>} onChanged
 * @param {function(Error): void} onError
 * @return {function(): void} stop watching
 */
export default function watchCertificateConfig(
  scheduledConfig,
  provider,
  configFileRepository,
  onChanged,
  onError,
) {
  const configName = scheduledConfig.getName();
  const scheduledFingerprint = getRenewalConfigurationFingerprint(
    scheduledConfig,
    provider,
  );
  let waitingForActiveConfig = provider === null;
  let isChecking = false;

  const notifyChanged = async (timer, currentConfig) => {
    try {
      const rescheduled = await onChanged(currentConfig);

      if (rescheduled === false) {
        waitingForActiveConfig = true;
      } else {
        clearInterval(timer);
      }
    } catch (e) {
      onError(e);
    }
  };

  const interval = setInterval(async () => {
    if (isChecking) {
      return;
    }

    isChecking = true;
    let currentConfig;

    try {
      currentConfig = configFileRepository.read().getConfig(configName);
    } catch (e) {
      if (e instanceof ConfigIsNotPresentError) {
        await notifyChanged(interval, null);
      } else {
        onError(e);
      }

      isChecking = false;
      return;
    }

    const isEnabled = currentConfig.get('platform.gateway.ssl.enabled');

    if ((!waitingForActiveConfig
      && isEnabled
      && currentConfig.get('platform.gateway.ssl.provider') === provider
      && getRenewalConfigurationFingerprint(currentConfig, provider) === scheduledFingerprint)
      || (waitingForActiveConfig && !isEnabled)) {
      isChecking = false;
      return;
    }

    await notifyChanged(interval, currentConfig);
    isChecking = false;
  }, CONFIG_REFRESH_INTERVAL_MS);

  return () => clearInterval(interval);
}
