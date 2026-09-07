import generateTenderdashNodeKey from './generateTenderdashNodeKey.js';
import deriveTenderdashNodeId from './deriveTenderdashNodeId.js';

const NODE_ID_PATH = 'platform.drive.tenderdash.node.id';
const NODE_KEY_PATH = 'platform.drive.tenderdash.node.key';

/**
 * @return {ensureTenderdashNodeKey}
 */
export default function ensureTenderdashNodeKeyFactory() {
  /**
   * Fill in a missing tenderdash node identity on a config.
   *
   * The interactive setup wizard is the only flow that collects a node key, so
   * a config assembled any other way (dashmate config create, non-interactive
   * setup, enabling platform on an existing node) carries
   * platform.drive.tenderdash.node.{id,key} as null, and node_key.json is
   * rendered with the literal string "null" - tenderdash panics at startup.
   *
   * Only the config it is given is changed. It runs before a config is saved
   * and before its templates are rendered, so config.json and node_key.json
   * agree on the identity and a restart reuses it. An existing key is never
   * touched, which is what makes running it at both points safe.
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

    const existingKey = config.get(NODE_KEY_PATH);

    if (existingKey !== null) {
      // The id is derivable, so a config carrying a key without one is
      // completed rather than rejected.
      if (config.get(NODE_ID_PATH) === null) {
        config.set(NODE_ID_PATH, deriveTenderdashNodeId(existingKey));
      }

      return;
    }

    // An evonode's node id is registered on chain in its ProRegTx. A key
    // generated here would start a healthy-looking node under an identity the
    // network does not know, instead of failing at startup. The wizard insists
    // on the registered key for a masternode and doctor reports a mismatch, so
    // this is left to the operator.
    if (config.get('core.masternode.enable') === true) {
      return;
    }

    const key = generateTenderdashNodeKey();

    config.set(NODE_ID_PATH, deriveTenderdashNodeId(key));
    config.set(NODE_KEY_PATH, key);
  }

  return ensureTenderdashNodeKey;
}
