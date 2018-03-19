const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');

use(dirtyChai);
use(sinonChai);

const SyncState = require('../../../../lib/sync/state/SyncState');
const SyncStateRepositoryChangeListener =
  require('../../../../lib/sync/state/repository/SyncStateRepositoryChangeListener');

const getBlockFixtures = require('../../../../lib/test/fixtures/getBlockFixtures');

describe('SyncStateRepositoryChangeListener', () => {
  let repositoryMock;
  let checkInterval;
  let changeListener;
  let timers;

  beforeEach(function beforeEach() {
    if (!this.sinon) {
      this.sinon = sinon.sandbox.create();
    } else {
      this.sinon.restore();
    }

    const syncState = new SyncState([], new Date());

    class SyncStateRepository { }
    repositoryMock = new SyncStateRepository();
    repositoryMock.fetch = this.sinon.stub();
    repositoryMock.fetch.returns(Promise.resolve(syncState));

    checkInterval = 10;
    changeListener = new SyncStateRepositoryChangeListener(repositoryMock, checkInterval);

    timers = sinon.useFakeTimers({ toFake: ['setInterval'] });
  });

  afterEach(() => {
    changeListener.stop();
  });

  it('should return repository', () => {
    expect(changeListener.getRepository()).to.be.equals(repositoryMock);
  });

  it('should not listen if already do that', async () => {
    expect(await changeListener.listen()).to.be.true();
    expect(await changeListener.listen()).to.be.false();
  });

  it('should listen changes every specified interval', async function it() {
    const changeHandler = this.sinon.stub();
    const errorHandler = this.sinon.stub();

    changeListener.on('change', changeHandler);
    changeListener.on('error', errorHandler);

    await changeListener.listen();

    timers.tick(checkInterval);

    expect(repositoryMock.fetch).to.be.calledTwice();

    expect(changeHandler).not.to.be.called();
    expect(errorHandler).not.to.be.called();
  });

  it('should emit "change" when repository data has changed', async function it() {
    const changeHandler = this.sinon.stub();
    const errorHandler = this.sinon.stub();

    changeListener.on('change', changeHandler);
    changeListener.on('error', errorHandler);

    await changeListener.listen();

    const blocks = getBlockFixtures();
    const newState = new SyncState(blocks, new Date());
    repositoryMock.fetch.returns(Promise.resolve(newState));

    timers.next();

    await new Promise((resolve) => {
      process.nextTick(() => {
        expect(changeHandler).to.be.calledOnce();
        expect(changeHandler).to.be.calledWith(newState);

        expect(errorHandler).not.to.be.called();

        resolve();
      });
    });
  });

  it('should emit "error" when error has occurred', async function it() {
    const changeHandler = this.sinon.stub();
    const errorHandler = this.sinon.stub();

    changeListener.on('change', changeHandler);
    changeListener.on('error', errorHandler);

    await changeListener.listen();

    const error = new Error();
    repositoryMock.fetch.returns(Promise.reject(error));

    timers.next();

    await new Promise((resolve) => {
      process.nextTick(() => {
        expect(errorHandler).to.be.calledOnce();
        expect(errorHandler).to.be.calledWith(error);

        expect(changeHandler).not.to.be.called();

        resolve();
      });
    });
  });

  it('should stop listen changes', async () => {
    await changeListener.listen();
    changeListener.stop();

    timers.next();

    expect(repositoryMock.fetch).to.be.calledOnce();
  });
});
