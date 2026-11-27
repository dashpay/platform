import killContainersSynchronously from '../../../src/docker/killContainersSynchronously.js';

describe('killContainersSynchronously', () => {
  // listr2 answers SIGINT with process.exit(), and after that only synchronous
  // work runs: a Core that setup started to read a registration would be left
  // running on Ctrl-C, with this node's operator key.
  it('should kill the containers before it returns', function it() {
    const spawnSync = this.sinon.stub().returns({ status: 0 });

    killContainersSynchronously(['a1b2', 'c3d4'], spawnSync);

    expect(spawnSync).to.have.been.calledOnceWithExactly(
      'docker',
      ['kill', 'a1b2', 'c3d4'],
      { stdio: 'ignore', timeout: 10000 },
    );
  });

  it('should not run docker when the command started no container', function it() {
    const spawnSync = this.sinon.stub();

    killContainersSynchronously([], spawnSync);

    expect(spawnSync).to.not.have.been.called();
  });

  // The asynchronous cleanup still runs after it and reports what it can.
  it('should not throw when docker cannot be run', function it() {
    const spawnSync = this.sinon.stub().returns({ error: new Error('spawn docker ENOENT') });

    expect(() => killContainersSynchronously(['a1b2'], spawnSync)).to.not.throw();
  });
});
