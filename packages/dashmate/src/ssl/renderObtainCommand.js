import chalk from 'chalk';
import { SAFE_ACTION, ISSUANCE_STATUS } from './renewalGuidance.js';
import renderConfigFlag from '../util/renderConfigFlag.js';

/**
 * The only thing allowed to put a certificate request in front of an operator.
 *
 * Every surface that reports on certificates had its own copy of this command,
 * and each one decided for itself whether printing it was safe. They did not
 * agree: a request appeared directly beneath the sentence withholding it, a
 * provider-switch ending printed one after the shared derivation had already
 * refused, and a node with an issuance outstanding was handed the command by a
 * branch that had never heard of the record.
 *
 * Fixing those one at a time did not converge - each round of review found more
 * of them - so the decision is not made per branch any more. A caller asks for
 * the command and is given either the command or the reason it is being
 * withheld, and cannot tell the difference without looking. A branch that
 * forgets to consult the guidance is now impossible rather than unlikely, and
 * `renderObtainCommand.spec.js` fails if the raw command reappears in any
 * surface that has guidance available.
 *
 * @param {Object} options
 * @param {string} options.configName
 * @param {Object} options.guidance - from deriveRenewalGuidance
 * @param {boolean} [options.force]
 * @param {string} [options.provider] - written into the command when given
 * @return {string} the command, or what to do instead
 */
export default function renderObtainCommand({
  configName, guidance, force = false, provider = 'letsencrypt',
}) {
  // A command with no node named is worse than no command: an operator pastes
  // it, and it runs against the default config or none. Refusing here means a
  // caller that forgets is a visible failure rather than a wrong instruction.
  if (!configName) {
    throw new Error('renderObtainCommand needs the config the command is for');
  }

  const cfg = renderConfigFlag(configName);
  const flags = `${provider ? ` --provider ${provider}` : ''}${force ? ' --force' : ''}`;
  const command = chalk`{bold.cyanBright dashmate ssl obtain ${cfg}${flags}}`;

  const { safeAction, issuanceStatus } = guidance;

  // The node still works and renewal comes back around by itself, so asking now
  // spends one of the few failed attempts the authority allows on a repair that
  // has not been made yet.
  if (safeAction === SAFE_ACTION.WAIT_AFTER_LOCAL_FIX) {
    return chalk`Fix the cause above. dashmate retries by itself - then check it worked:
{bold.cyanBright dashmate doctor ${cfg}}`;
  }

  if (safeAction !== SAFE_ACTION.DO_NOT_OBTAIN) {
    return command;
  }

  // Withheld, and for which of two reasons. Saying "could not be saved" when
  // dashmate does not know whether a certificate exists is a claim it cannot
  // make, and the operator's next step differs.
  if (issuanceStatus === ISSUANCE_STATUS.SPENT) {
    return chalk`Do not obtain one - a certificate was already issued and could not be saved,
so asking again spends another. Send a report instead:
{bold.cyanBright dashmate doctor report ${cfg}}`;
  }

  if (issuanceStatus === ISSUANCE_STATUS.UNCERTAIN) {
    return chalk`Do not obtain one yet - an earlier attempt may already have been issued a
certificate without dashmate seeing it. Send a report instead:
{bold.cyanBright dashmate doctor report ${cfg}}`;
  }

  return chalk`Send a report to Dash support:
{bold.cyanBright dashmate doctor report ${cfg}}`;
}
