import MuteOneLineError from './MuteOneLineError.js';

/**
 * Print an error's message as it is, and return it muted for the command to
 * throw
 *
 * oclif hard-wraps the errors it prints to the terminal width. A message that
 * gives commands to copy would have them split across lines, and each half
 * pasted alone runs something else.
 *
 * @param {Error} error
 * @return {MuteOneLineError}
 */
export default function printUnwrapped(error) {
  process.stderr.write(`${error.message}\n`);

  return new MuteOneLineError(error);
}
