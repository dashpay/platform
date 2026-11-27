import quoteShellArgument from './quoteShellArgument.js';

/**
 * Render the --config flag for a command an operator is meant to copy and run.
 *
 * Every command dashmate prints carries this. Without it an operator running
 * several configs who pastes a bare command obtains a certificate for,
 * restarts, or bypasses a check on a different node - the command falls back to
 * the default config when --config is absent.
 *
 * @param {string} configName
 * @return {string}
 */
export default function renderConfigFlag(configName) {
  return `--config ${quoteShellArgument(configName)}`;
}
