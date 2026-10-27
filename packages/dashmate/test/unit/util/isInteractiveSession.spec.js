import isInteractiveSession from '../../../src/util/isInteractiveSession.js';

describe('isInteractiveSession', () => {
  const TTY = { isTTY: true };
  // Node reports a stream that is not a terminal as `undefined`, never `false`,
  // so every rule has to survive the absent property rather than a boolean.
  const NOT_TTY = {};

  /**
   * @param {Object} [overrides]
   * @return {boolean}
   */
  function detect(overrides = {}) {
    return isInteractiveSession({
      flags: {},
      env: {},
      stdin: TTY,
      stdout: TTY,
      ...overrides,
    });
  }

  // The table of environments the detection has to place correctly. A wrong
  // answer one way breaks CI and Ansible; a wrong answer the other way hangs an
  // unattended upgrade on a node the documented flow has already stopped.
  const environments = [
    ['operator at a terminal', { stdin: TTY, stdout: TTY, env: {} }, true],
    ['dashmate update > log 2>&1', { stdin: TTY, stdout: NOT_TTY, env: {} }, false],
    ['dashmate update | tee log', { stdin: TTY, stdout: NOT_TTY, env: {} }, false],
    ['dashmate update < /dev/null', { stdin: NOT_TTY, stdout: TTY, env: {} }, false],
    ['cron', { stdin: NOT_TTY, stdout: NOT_TTY, env: {} }, false],
    ['systemd with StandardInput=null', { stdin: NOT_TTY, stdout: NOT_TTY, env: {} }, false],
    ['Ansible command/shell', { stdin: NOT_TTY, stdout: NOT_TTY, env: {} }, false],
    ['Ansible become with a pty', { stdin: TTY, stdout: TTY, env: {} }, true],
    ['GitHub Actions', { stdin: NOT_TTY, stdout: NOT_TTY, env: { CI: 'true' } }, false],
    ['docker exec', { stdin: NOT_TTY, stdout: NOT_TTY, env: {} }, false],
    ['docker exec -it', { stdin: TTY, stdout: TTY, env: {} }, true],
    // Resolves interactive with the explanation in a file nobody is watching,
    // which is why every prompt header has to carry its own context.
    ['dashmate update 2> log', { stdin: TTY, stdout: TTY, env: {} }, true],
  ];

  environments.forEach(([name, streams, expected]) => {
    it(`should report ${name} as ${expected ? 'interactive' : 'non-interactive'}`, () => {
      expect(detect(streams)).to.equal(expected);
    });
  });

  // An operator who says "never prompt" has to be obeyed even at a terminal:
  // it is the only thing that saves a playbook that allocates a pty, which is
  // otherwise indistinguishable from a human.
  it('should never prompt when the operator asked for it, even at a terminal', () => {
    expect(detect({ flags: { 'non-interactive': true } })).to.be.false();
  });

  // The environment variable exists so automation can be armed before the
  // binary that understands the flag is installed.
  it('should never prompt when the environment asks for it', () => {
    expect(detect({ env: { DASHMATE_NON_INTERACTIVE: '1' } })).to.be.false();
  });

  it('should ignore the environment variable when it is switched off', () => {
    expect(detect({ env: { DASHMATE_NON_INTERACTIVE: '0' } })).to.be.true();
    expect(detect({ env: { DASHMATE_NON_INTERACTIVE: 'false' } })).to.be.true();
    expect(detect({ env: { DASHMATE_NON_INTERACTIVE: '' } })).to.be.true();
  });

  // Prompt chrome is written to stdout, so prompting under JSON output would
  // corrupt the one parseable document the caller asked for.
  it('should never prompt when the output is meant for a machine', () => {
    expect(detect({ flags: { format: 'json' } })).to.be.false();
  });

  it('should treat any CI value that is not switched off as a machine', () => {
    ['true', 'TRUE', 'True', '1', 'yes'].forEach((value) => {
      expect(detect({ env: { CI: value } }), value).to.be.false();
    });
  });

  // A human debugging on a box that exports CI needs a way back, and this is
  // the documented one.
  it('should let CI=0 hand the terminal back to a human', () => {
    ['0', 'false', 'FALSE', ''].forEach((value) => {
      expect(detect({ env: { CI: value } }), value).to.be.true();
    });
  });

  // The explicit instruction outranks the heuristic, not the other way round.
  it('should let an explicit flag outrank CI', () => {
    expect(detect({ flags: { 'non-interactive': true }, env: { CI: '0' } })).to.be.false();
  });

  // A stream that is not a terminal reports `undefined`. Tidying the check into
  // `=== false` would classify every pipe and every cron run as interactive.
  it('should treat an absent isTTY as not a terminal', () => {
    expect(NOT_TTY.isTTY).to.be.undefined();
    expect(detect({ stdin: NOT_TTY })).to.be.false();
    expect(detect({ stdout: NOT_TTY })).to.be.false();
  });

  // oclif replaces the streams it manages, so a value captured when the module
  // loaded describes the wrong process by the time the gate asks.
  it('should read the streams at call time', () => {
    const stdin = { isTTY: true };
    const stdout = { isTTY: true };

    expect(isInteractiveSession({ flags: {}, env: {}, stdin, stdout })).to.be.true();

    stdin.isTTY = undefined;

    expect(isInteractiveSession({ flags: {}, env: {}, stdin, stdout })).to.be.false();
  });
});
