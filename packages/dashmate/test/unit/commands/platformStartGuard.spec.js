import StartCommand from '../../../src/commands/start.js';
import RestartCommand from '../../../src/commands/restart.js';
import ReindexCommand from '../../../src/commands/core/reindex.js';
import GroupStartCommand from '../../../src/commands/group/start.js';
import GroupRestartCommand from '../../../src/commands/group/restart.js';
import Config from '../../../src/config/Config.js';
import MuteOneLineError from '../../../src/oclif/errors/MuteOneLineError.js';

// Each command that starts Platform refuses a masternode without its
// registered identity before it does anything: a restart or a reindex that
// stopped the node or rendered its files first would leave it down.
describe('commands that start Platform', () => {
  let config;
  let refusal;
  let assertPlatformCanStart;
  let work;
  let stderr;

  beforeEach(function beforeEach() {
    config = new Config('testnet', { group: 'local' }, true);
    refusal = new Error('Cannot start Platform of testnet: this masternode has no Platform node key.');
    assertPlatformCanStart = this.sinon.stub().throws(refusal);
    work = this.sinon.stub();

    stderr = '';
    this.sinon.stub(process.stderr, 'write').callsFake((chunk) => {
      stderr += chunk;

      return true;
    });
  });

  const flags = {
    verbose: false, platform: false, force: false, safe: false, detach: false,
  };

  [
    ['start', () => new StartCommand().runWithDependencies(
      {},
      flags,
      {},
      work,
      work,
      config,
      assertPlatformCanStart,
    )],
    ['restart', () => new RestartCommand().runWithDependencies(
      {},
      flags,
      {},
      work,
      config,
      assertPlatformCanStart,
    )],
    ['core reindex', () => new ReindexCommand().runWithDependencies(
      {},
      flags,
      config,
      work,
      assertPlatformCanStart,
    )],
    ['group start', () => new GroupStartCommand().runWithDependencies(
      {},
      flags,
      {},
      work,
      [config],
      work,
      assertPlatformCanStart,
    )],
    ['group restart', () => new GroupRestartCommand().runWithDependencies(
      {},
      flags,
      {},
      work,
      work,
      [config],
      assertPlatformCanStart,
    )],
  ].forEach(([name, runCommand]) => {
    // oclif would hard-wrap the message and split the commands in it, so it
    // is printed as it is and the error is muted.
    it(`should refuse in ${name} before any work, printing the refusal unwrapped`, async () => {
      const error = await runCommand().then(() => null, (e) => e);

      expect(error).to.be.an.instanceOf(MuteOneLineError);
      expect(error.getError()).to.equal(refusal);
      expect(stderr).to.equal(`${refusal.message}\n`);
      expect(assertPlatformCanStart).to.have.been.calledWith(config);
      expect(work).to.not.have.been.called();
    });
  });
});
