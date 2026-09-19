import generateTenderdashNodeKey from './generateTenderdashNodeKey.js';
import deriveTenderdashNodeId from './deriveTenderdashNodeId.js';

/**
 * Complete the in-flight identity before saving or rendering, without nested saves.
 * @param {Config} config
 */
export default function ensureTenderdashNodeKey(config) {
  // The base template is cloned into new configs, which must not share an identity.
  if (config.get('platform.enable') !== true || config.getName() === 'base') {
    return;
  }

  const options = config.getStoredOptions();
  const { node } = options.platform.drive.tenderdash;
  const { key = null, id = null } = node;
  if (key !== null && id !== null) {
    return;
  }
  // Masternodes must use the identity registered on chain by their operator.
  if (key === null && config.get('core.masternode.enable') === true) {
    return;
  }

  node.key ??= generateTenderdashNodeKey();
  node.id = deriveTenderdashNodeId(node.key);
  // Hydration already validated (or explicitly bypassed validation for recovery).
  // Only these trusted identity fields change; do not revalidate unrelated options.
  config.setOptions(options, true);
}
