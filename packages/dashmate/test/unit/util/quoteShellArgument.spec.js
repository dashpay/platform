import { execFileSync } from 'child_process';
import hasbin from 'hasbin';
import quoteShellArgument from '../../../src/util/quoteShellArgument.js';

describe('quoteShellArgument', () => {
  /**
   * @param {string} shell
   * @param {string} quoted
   * @return {string} what the shell makes of the quoted argument
   */
  function evaluateIn(shell, quoted) {
    return execFileSync(shell, ['-c', `printf '%s' ${quoted}`], { encoding: 'utf8' });
  }

  it('should leave a value a shell passes through untouched as it is', () => {
    expect(quoteShellArgument('/home/user/.dashmate/testnet/platform')).to.equal('/home/user/.dashmate/testnet/platform');
  });

  // A quote inside single quotes would end them, and the rest of the value
  // would be read as shell syntax.
  it('should keep a value with a single quote one argument', () => {
    const value = "/home/o'brien/.dashmate/testnet";

    const quoted = quoteShellArgument(value);

    expect(quoted).to.equal("'/home/o'\\''brien/.dashmate/testnet'");
    expect(evaluateIn('sh', quoted)).to.equal(value);
  });

  // A line break inside single quotes is kept, so the printed command would
  // take two lines, the second reading as a command of its own.
  it('should keep a value with control characters on one line', function test() {
    // Harmless if it ever ran: the shell evaluates it below.
    const value = "/home/user/locked\nprintf injected\u001b[2K'\\";

    const quoted = quoteShellArgument(value);

    expect(quoted).to.equal("$'/home/user/locked\\012printf injected\\033[2K\\'\\\\'");
    expect(quoted).to.not.include('\n').and.not.include('\u001b');

    if (!hasbin.sync('bash')) {
      this.skip();
    }

    expect(evaluateIn('bash', quoted)).to.equal(value);
  });
});
