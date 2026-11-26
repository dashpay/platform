import { execFileSync } from 'child_process';
import hasbin from 'hasbin';
import quoteShellArgument, { isShellQuotable } from '../../../src/util/quoteShellArgument.js';

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

  // The printed commands are pasted into whatever shell the operator has,
  // dash (Debian's and Ubuntu's /bin/sh) included, which knows no quoting
  // beyond POSIX.
  ['sh', 'dash'].forEach((shell) => {
    it(`should keep a value one argument for ${shell}`, function test() {
      if (!hasbin.sync(shell)) {
        this.skip();
      }

      const value = "/home/o'brien/my node/$HOME `id` \\n !x ~ * ü";

      expect(evaluateIn(shell, quoteShellArgument(value))).to.equal(value);
    });
  });

  // A line break inside single quotes is kept, so the printed command would
  // take two lines, the second reading as a command of its own. Dollar-single
  // quotes escape it, but dash reads them as a dollar sign and a quoted
  // string, and the command would act on another path.
  it('should refuse a value with control characters', () => {
    const value = '/home/user/locked\nprintf injected\u001b[2K';

    expect(isShellQuotable(value)).to.be.false();
    expect(isShellQuotable("/home/o'brien")).to.be.true();
    expect(() => quoteShellArgument(value)).to.throw('control characters');
  });
});
