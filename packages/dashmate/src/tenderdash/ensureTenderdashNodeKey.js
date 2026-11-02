import generateTenderdashNodeKey from './generateTenderdashNodeKey.js';
import deriveTenderdashNodeId from './deriveTenderdashNodeId.js';

/**
 * Complete the in-flight identity before saving or rendering, without nested saves.
 * @param {Config} config
 */
export default function ensureTenderdashNodeKey(config) {
  // Read stored options rather than config.get(): a recovery reset bypasses
  // validation, so nested objects may be missing and must not abort the save.
  const options = config.getStoredOptions();

  // The base template is cloned into new configs, which must not share an identity.
  if (options.platform?.enable !== true || config.getName() === 'base') {
    return;
  }

  options.platform.drive ??= {};
  options.platform.drive.tenderdash ??= {};
  options.platform.drive.tenderdash.node ??= {};
  const { node } = options.platform.drive.tenderdash;
  const { key = null, id = null } = node;
  if (key !== null && id !== null) {
    return;
  }
  // Masternodes must use the identity registered on chain by their operator.
  if (key === null && options.core?.masternode?.enable === true) {
    return;
  }

  node.key ??= generateTenderdashNodeKey();
  node.id = deriveTenderdashNodeId(node.key);
  // Hydration already validated (or explicitly bypassed validation for recovery).
  // Only these trusted identity fields change; do not revalidate unrelated options.
  config.setOptions(options, true);
}
