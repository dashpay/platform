import fs from 'fs';
import os from 'os';
import path from 'path';
import createLegacyReportCleanupTasks from '../../../../../src/listr/tasks/doctor/legacyReportCleanupTasks.js';

describe('legacyReportCleanupTasks', () => {
  const legacyName = 'dashmate-report-2026-01-01T00:00:00.000Z';

  let tempDir;

  /**
   * @param {string} name
   * @return {string}
   */
  function makeStagingDir(name) {
    const dir = path.join(tempDir, name);

    fs.mkdirSync(path.join(dir, 'core'), { recursive: true });
    fs.writeFileSync(path.join(dir, 'date.txt'), name.slice('dashmate-report-'.length));
    fs.writeFileSync(path.join(dir, 'core', 'stdOut.txt'), 'log line\n');

    return dir;
  }

  /**
   * Run both tasks the way the report command does, answering the removal
   * question with `answer`
   *
   * @param {Object} sinon
   * @param {boolean} answer
   * @return {Promise<{ctx: Object, find: Object, remove: Object}>}
   */
  async function run(sinon, answer) {
    const [findTask, removeTask] = createLegacyReportCleanupTasks(tempDir);
    const ctx = {};

    const find = { skip: sinon.stub(), output: undefined };
    await findTask.task(ctx, find);

    const remove = { skip: sinon.stub(), output: undefined, prompt: sinon.stub().resolves(answer) };

    if (removeTask.enabled(ctx)) {
      await removeTask.task(ctx, remove);
    }

    return { ctx, find, remove };
  }

  beforeEach(() => {
    tempDir = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'dashmate-legacy-cleanup-test-')));
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  // A directory that looks like what an earlier version staged can also be a
  // report the operator unpacked there and kept, so it is only removed once
  // the operator, shown exactly what will go, agrees
  it('should list the directories it would remove and leave them when the operator says no', async function it() {
    const dir = makeStagingDir(legacyName);

    const { remove } = await run(this.sinon, false);

    expect(remove.prompt).to.have.been.calledOnce();
    expect(remove.prompt.firstCall.args[0].header).to.contain(dir);
    expect(fs.existsSync(dir)).to.be.true();
  });

  it('should remove the listed directories when the operator says yes', async function it() {
    const dir = makeStagingDir(legacyName);

    const { remove } = await run(this.sinon, true);

    expect(fs.existsSync(dir)).to.be.false();
    expect(remove.output).to.contain(`Removed ${dir}`);
  });

  it('should not ask when there is nothing to remove', async function it() {
    const { find, remove } = await run(this.sinon, true);

    expect(find.skip).to.have.been.calledOnce();
    expect(remove.prompt).to.not.have.been.called();
  });

  // The operator needs to tell a directory that still needs their attention
  // apart from a temp directory that could not be looked in at all
  it('should say which directories it left, and whether it closed them', async function it() {
    const dir = makeStagingDir(legacyName);
    fs.writeFileSync(path.join(dir, 'notes.txt'), 'mine');

    const { find, remove } = await run(this.sinon, true);

    expect(find.output).to.match(new RegExp(`^Left ${dir.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}, closed to other users: `));
    expect(remove.prompt).to.not.have.been.called();
  });

  it('should report a temp directory it could not look in apart from directories it left', async function it() {
    makeStagingDir(legacyName);
    fs.chmodSync(tempDir, 0o777);

    const { find } = await run(this.sinon, true);

    expect(find.output).to.match(/^Could not look in .* for files left by earlier reports: /);
    expect(find.output).to.not.contain('Left ');
  });
});
