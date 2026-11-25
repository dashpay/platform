import { Parser } from '@oclif/core';
import GroupRestartCommand from '../../../../src/commands/group/restart.js';

describe('GroupRestartCommand', () => {
  async function parseSafe(argv) {
    const { flags } = await Parser.parse(argv, { flags: GroupRestartCommand.flags });

    return flags.safe;
  }

  it('should parse --safe so the restart waits for a safe DKG window', async () => {
    expect(await parseSafe(['--safe'])).to.equal(true);
    expect(await parseSafe(['-s'])).to.equal(true);
  });

  it('should not wait for a DKG window by default', async () => {
    expect(await parseSafe([])).to.equal(false);
  });
});
