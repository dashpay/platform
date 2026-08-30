import generateTenderdashNodeKey from './generateTenderdashNodeKey.js';
import deriveTenderdashNodeId from './deriveTenderdashNodeId.js';

/**
 * @param {ConfigFileJsonRepository} configFileRepository
 * @return {ensureTenderdashNodeKey}
 */
export default function ensureTenderdashNodeKeyFactory(configFileRepository) {
  /**
   * Persist node identity values into the stored copy of the config, so a
   * restart reuses the same identity instead of generating a new one. The
   * config file is re-read under its lock, and a value that appeared there in
   * the meantime wins over the one generated here.
   *
   * For a command holding the lock across its run this is an intermediate
   * write: it persists the identity ahead of the command's own final save,
   * without that command's other pending in-memory edits. Those still land
   * with the final save; only if the command dies first does the identity
   * outlive them - which is the point, since the rendered files already
   * reference it.
   *
   * @param {Config} config
   * @param {string} id
   * @param {string} key
   * @returns {void}
   */
  function persistNodeIdentity(config, id, key) {
    configFileRepository.update((configFile) => {
      // A config not stored yet (a preset being set up) is persisted by the
      // command that created it once it saves the config file it holds.
      if (!configFile.isConfigExists(config.getName())) {
        return;
      }

      const storedConfig = configFile.getConfig(config.getName());
      const storedKey = storedConfig.get('platform.drive.tenderdash.node.key');

      if (storedKey === null || storedKey === key) {
        storedConfig.set('platform.drive.tenderdash.node.id', id);
        storedConfig.set('platform.drive.tenderdash.node.key', key);
      } else {
        // Another process stored a different identity first; render with
        // theirs, deriving the id when it is not stored either.
        config.set(
          'platform.drive.tenderdash.node.id',
          storedConfig.get('platform.drive.tenderdash.node.id') ?? deriveTenderdashNodeId(storedKey),
        );
        config.set('platform.drive.tenderdash.node.key', storedKey);
      }
    });
  }

  /**
   * Fill in a missing tenderdash node identity before service configs are
   * rendered.
   *
   * The interactive setup wizard is the only flow that collects a node key, so
   * a config assembled any other way (dashmate config create, non-interactive
   * setup, enabling platform on an existing node) reaches template rendering
   * with platform.drive.tenderdash.node.{id,key} still null, and node_key.json
   * is written with the literal string "null" - tenderdash panics at startup.
   * An existing key is never touched.
   *
   * @typedef {ensureTenderdashNodeKey}
   * @param {Config} config
   * @returns {void}
   */
  function ensureTenderdashNodeKey(config) {
    if (config.get('platform.enable') !== true) {
      return;
    }

    // The base config is a template: a key generated for it would be cloned
    // into every config created from it, and those must not share an identity.
    if (config.getName() === 'base') {
      return;
    }

    const existingKey = config.get('platform.drive.tenderdash.node.key');

    if (existingKey !== null) {
      // The id is derivable, so a config carrying a key without one is
      // completed rather than rejected.
      if (config.get('platform.drive.tenderdash.node.id') === null) {
        const id = deriveTenderdashNodeId(existingKey);

        config.set('platform.drive.tenderdash.node.id', id);

        persistNodeIdentity(config, id, existingKey);
      }

      return;
    }

    const key = generateTenderdashNodeKey();
    const id = deriveTenderdashNodeId(key);

    config.set('platform.drive.tenderdash.node.id', id);
    config.set('platform.drive.tenderdash.node.key', key);

    persistNodeIdentity(config, id, key);
  }

  return ensureTenderdashNodeKey;
}
