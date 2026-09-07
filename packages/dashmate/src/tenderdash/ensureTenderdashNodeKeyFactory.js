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
   * Only the setup wizard collects a node key, so a config assembled any other
   * way carries node.{id,key} as null and node_key.json renders the literal
   * string "null" - tenderdash panics at startup. Runs both before a config is
   * saved and before its templates are rendered, so config.json and
   * node_key.json agree; an existing key is never touched, which is what makes
   * running it at both points safe.
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
      // The id is derivable, so complete a key missing one rather than reject it
      if (config.get(NODE_ID_PATH) === null) {
        config.set(NODE_ID_PATH, deriveTenderdashNodeId(existingKey));
      }

      return;
    }

    // An evonode's node id is registered on chain in its ProRegTx, so a key
    // generated here would start a healthy-looking node under an identity the
    // network does not know instead of failing at startup. Left to the operator.
    if (config.get('core.masternode.enable') === true) {
      return;
    }

    const key = generateTenderdashNodeKey();

    config.set(NODE_ID_PATH, deriveTenderdashNodeId(key));
    config.set(NODE_KEY_PATH, key);
  }

  return ensureTenderdashNodeKey;
}
