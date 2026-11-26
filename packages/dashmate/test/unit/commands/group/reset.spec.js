import { Listr } from 'listr2';
import GroupResetCommand from '../../../../src/commands/group/reset.js';

describe('GroupResetCommand', () => {
  let members;
  let resetNodeTask;

  beforeEach(function beforeEach() {
    members = ['local_1', 'local_2'].map((name) => ({
      getName: () => name,
      get: (optionPath) => (optionPath === 'group' ? 'local' : undefined),
    }));

    resetNodeTask = this.sinon.stub().callsFake(() => new Listr([]));
    resetNodeTask.assertCanBeReset = this.sinon.stub().resolves();
  });

  function runGroupReset(flags) {
    return GroupResetCommand.prototype.runWithDependencies.call(
      {},
      {},
      {
        verbose: false,
        hard: false,
        force: true,
        platform: false,
        ...flags,
      },
      resetNodeTask,
      members,
      () => new Listr([]),
      () => new Listr([]),
    );
  }

  // Members are reset one after another. A member found unable to be reset
  // only after the others were leaves them half reset, with service files
  // Compose needs already gone.
  it('should check every member before it resets any', async () => {
    resetNodeTask.assertCanBeReset.withArgs(members[1]).rejects(new Error('cannot reset local_2'));

    const error = await runGroupReset({ hard: true }).then(() => null, (e) => e);

    expect(error.getError().message).to.equal('cannot reset local_2');
    expect(resetNodeTask.assertCanBeReset).to.have.been.calledTwice();
    expect(resetNodeTask).to.not.have.been.called();
  });

  // The check judges removing the local group's configs, which a hard group
  // reset does, so it must see that the reset will remove them.
  it('should tell the check that a hard reset of the local group removes the configs', async () => {
    await runGroupReset({ hard: true, platform: true });

    expect(resetNodeTask.assertCanBeReset.firstCall.args[1]).to.include({
      isHardReset: true,
      isPlatformOnlyReset: true,
      removeConfig: true,
    });
    expect(resetNodeTask).to.have.been.calledTwice();
  });
});
