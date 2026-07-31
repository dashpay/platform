import ConfigIsNotPresentError from '../config/errors/ConfigIsNotPresentError.js';

export const CONFIG_REFRESH_INTERVAL_MS = 60 * 1000;

/**
 * Watch the local config for changes that make a scheduled renewal obsolete.
 * Without this poll, a provider switch made after a distant renewal was scheduled
 * would not be handed to the new provider until the old job eventually fired.
 *
 * @param {string} configName
 * @param {string} provider
 * @param {ConfigFileJsonRepository} configFileRepository
 * @param {function(Config|null): Promise<boolean|void>} onChanged
 * @param {function(Error): void} onError
 * @return {function(): void} stop watching
 */
export default function watchCertificateConfig(
  configName,
  provider,
  configFileRepository,
  onChanged,
  onError,
) {
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
      && currentConfig.get('platform.gateway.ssl.provider') === provider)
      || (waitingForActiveConfig && !isEnabled)) {
      isChecking = false;
      return;
    }

    await notifyChanged(interval, currentConfig);
    isChecking = false;
  }, CONFIG_REFRESH_INTERVAL_MS);

  return () => clearInterval(interval);
}
