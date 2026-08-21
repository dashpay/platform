/**
 * Whether an environment variable carrying a boolean is switched on.
 *
 * An unset variable, "0", "false" and an empty value all mean off. The
 * comparison is case-folded because CI systems write TRUE, True and true
 * interchangeably and all three mean the same thing.
 *
 * @param {string|undefined} value
 * @return {boolean}
 */
function isEnvironmentFlagSet(value) {
  if (value === undefined || value === null) {
    return false;
  }

  const normalized = String(value).trim().toLowerCase();

  return normalized !== '' && normalized !== '0' && normalized !== 'false';
}

/**
 * Decide whether this process may ask the operator a question.
 *
 * The answer is fail-closed: everything that is not demonstrably a human at a
 * terminal is treated as automation. A wrong "non-interactive" answer reports a
 * problem and exits without changing anything, while a wrong "interactive"
 * answer waits for a keystroke that never arrives - and the documented upgrade
 * procedure stops the node before this runs, so that wait happens with the node
 * already down.
 *
 * The streams are read on every call rather than captured when the module
 * loads: oclif replaces the streams it manages, so a captured value can
 * describe a state that no longer holds. `isTTY` is `undefined` rather than
 * `false` on a stream that is not a terminal, which is why every test here is
 * `!== true`.
 *
 * @param {Object} [options]
 * @param {Object} [options.flags] - parsed command flags
 * @param {Object} [options.env]
 * @param {Object} [options.stdin]
 * @param {Object} [options.stdout]
 * @return {boolean}
 */
export default function isInteractiveSession({
  flags = {},
  env = process.env,
  stdin = process.stdin,
  stdout = process.stdout,
} = {}) {
  // An explicit instruction from the operator outranks every heuristic below,
  // including CI. The environment variable exists because a playbook cannot
  // carry a flag the currently installed binary would reject, so automation can
  // be armed before the upgrade rather than after it.
  if (flags?.['non-interactive'] === true
    || isEnvironmentFlagSet(env?.DASHMATE_NON_INTERACTIVE)) {
    return false;
  }

  // Prompt chrome is written to stdout, so prompting here would corrupt the
  // single parseable document the caller asked for.
  if (flags?.format === 'json') {
    return false;
  }

  // Every major CI system sets this, and some of them allocate a pty, which
  // defeats the stream checks below. A human debugging on a box that exports it
  // gets back to prompts with CI=0.
  if (isEnvironmentFlagSet(env?.CI)) {
    return false;
  }

  // No keystroke can ever arrive.
  if (stdin?.isTTY !== true) {
    return false;
  }

  // A prompt nobody can see is indistinguishable from a hang. This also
  // classifies `| tee` as automation, which is the conservative side of a real
  // trade-off: keying only on stdin would let `> log` hang in silence.
  if (stdout?.isTTY !== true) {
    return false;
  }

  return true;
}
