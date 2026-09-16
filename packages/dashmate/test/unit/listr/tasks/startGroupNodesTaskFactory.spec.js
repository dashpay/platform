import startGroupNodesTaskFactory from '../../../../src/listr/tasks/startGroupNodesTaskFactory.js';

const WAIT_FOR_NODES_TIMEOUT = 60 * 5 * 1000;

function createDeferred() {
  let resolve;
  const promise = new Promise((resolvePromise) => {
    resolve = resolvePromise;
  });

  return { promise, resolve };
}

describe('startGroupNodesTaskFactory', () => {
  function createConfig(sinon, name, rpcPort, network, minerEnabled = false) {
    const values = {
      'core.miner.enable': minerEnabled,
      'core.miner.address': 'yN6Q6xj3Y9SuZ9pY4FAP8Zn46G7N8sCqzF',
      'core.miner.interval': 60,
      'core.rpc.port': rpcPort,
      'core.rpc.users.dashmate.password': `${name}-password`,
      'dashmate.helper.docker.build.enabled': false,
      network,
      'platform.enable': false,
    };

    return {
      get: sinon.stub().callsFake((path) => values[path]),
      getName: sinon.stub().returns(name),
      set: sinon.stub(),
    };
  }

  function createFactory(sinon, overrides = {}) {
    const dependencies = {
      buildServicesTask: sinon.stub(),
      createRpcClient: sinon.stub().callsFake((options) => ({ options })),
      docker: {},
      dockerCompose: {
        execCommand: sinon.stub().resolves(),
      },
      getConnectionHost: sinon.stub().callsFake(
        async (config) => `${config.getName()}.test`,
      ),
      startNodeTask: sinon.stub().resolves(),
      waitForCorePeersConnected: sinon.stub().resolves(),
      waitForNodeToBeReadyTask: sinon.stub(),
      waitForNodesToHaveTheSameHeight: sinon.stub().resolves(),
      ...overrides,
    };

    const startGroupNodesTask = startGroupNodesTaskFactory(
      dependencies.dockerCompose,
      dependencies.waitForCorePeersConnected,
      dependencies.waitForNodesToHaveTheSameHeight,
      dependencies.createRpcClient,
      dependencies.docker,
      dependencies.startNodeTask,
      dependencies.waitForNodeToBeReadyTask,
      dependencies.buildServicesTask,
      dependencies.getConnectionHost,
    );

    return { dependencies, startGroupNodesTask };
  }

  it('should wait for every Core node to converge before starting the miner', async function shouldWaitForConvergence() {
    const configs = [
      createConfig(this.sinon, 'local_seed', 19998, 'local', true),
      createConfig(this.sinon, 'local_1', 20002, 'local'),
    ];
    const convergence = createDeferred();
    const peerWaits = configs.map(() => createDeferred());
    const allPeersStarted = createDeferred();
    const firstRelevantCall = createDeferred();
    const peerClients = [];

    const { dependencies, startGroupNodesTask } = createFactory(this.sinon, {
      dockerCompose: {
        execCommand: this.sinon.stub().callsFake(async () => {
          firstRelevantCall.resolve('miner');
        }),
      },
      waitForCorePeersConnected: this.sinon.stub().callsFake((rpcClient) => {
        peerClients.push(rpcClient);
        if (peerClients.length === configs.length) {
          allPeersStarted.resolve();
        }

        return peerWaits[peerClients.length - 1].promise;
      }),
      waitForNodesToHaveTheSameHeight: this.sinon.stub().callsFake(() => {
        firstRelevantCall.resolve('convergence');
        return convergence.promise;
      }),
    });

    const runPromise = startGroupNodesTask(configs).run({
      waitForReadiness: false,
    });

    try {
      await allPeersStarted.promise;
      expect(dependencies.waitForNodesToHaveTheSameHeight).to.not.have.been.called();
      expect(dependencies.dockerCompose.execCommand).to.not.have.been.called();

      peerWaits[0].resolve();
      await Promise.resolve();
      expect(dependencies.waitForNodesToHaveTheSameHeight).to.not.have.been.called();

      peerWaits[1].resolve();
      expect(await firstRelevantCall.promise).to.equal('convergence');
      expect(peerClients).to.have.lengthOf(configs.length);
      expect(dependencies.dockerCompose.execCommand).to.not.have.been.called();

      convergence.resolve();
      await runPromise;

      expect(dependencies.createRpcClient).to.have.callCount(configs.length);
      expect(dependencies.createRpcClient.getCalls().map(({ args }) => args[0]))
        .to.have.deep.members([
          {
            port: 19998,
            user: 'dashmate',
            pass: 'local_seed-password',
            host: 'local_seed.test',
          },
          {
            port: 20002,
            user: 'dashmate',
            pass: 'local_1-password',
            host: 'local_1.test',
          },
        ]);

      const rpcClients = dependencies.createRpcClient
        .getCalls()
        .map(({ returnValue }) => returnValue);
      expect(peerClients).to.have.deep.members(rpcClients);
      expect(dependencies.waitForNodesToHaveTheSameHeight)
        .to.have.been.calledOnceWithExactly(rpcClients, WAIT_FOR_NODES_TIMEOUT);
      expect(dependencies.dockerCompose.execCommand).to.have.been.calledOnce();
    } finally {
      peerWaits.forEach(({ resolve }) => resolve());
      convergence.resolve();
      await runPromise.catch(() => {});
    }
  });

  it('should not start the miner when Core convergence fails', async function shouldStopOnFailure() {
    const configs = [
      createConfig(this.sinon, 'local_seed', 19998, 'local', true),
      createConfig(this.sinon, 'local_1', 20002, 'local'),
    ];
    const convergenceError = new Error('Core tips do not match');
    const { dependencies, startGroupNodesTask } = createFactory(this.sinon, {
      waitForNodesToHaveTheSameHeight: this.sinon.stub().rejects(convergenceError),
    });

    await expect(startGroupNodesTask(configs).run({ waitForReadiness: false }))
      .to.be.rejectedWith(convergenceError);

    expect(dependencies.dockerCompose.execCommand).to.not.have.been.called();
  });

  it('should not wait for Core convergence without a local miner', async function shouldSkipWithoutMiner() {
    const configs = [
      createConfig(this.sinon, 'local_seed', 19998, 'local'),
      createConfig(this.sinon, 'local_1', 20002, 'local'),
    ];
    const { dependencies, startGroupNodesTask } = createFactory(this.sinon);

    await startGroupNodesTask(configs).run({ waitForReadiness: false });

    expect(dependencies.createRpcClient).to.not.have.been.called();
    expect(dependencies.waitForNodesToHaveTheSameHeight).to.not.have.been.called();
    expect(dependencies.dockerCompose.execCommand).to.not.have.been.called();
  });

  it('should not wait for Core convergence outside the local network', async function shouldSkipOutsideLocalNetwork() {
    const configs = [
      createConfig(this.sinon, 'testnet', 19998, 'testnet', true),
    ];
    const { dependencies, startGroupNodesTask } = createFactory(this.sinon);

    await startGroupNodesTask(configs).run({ waitForReadiness: false });

    expect(dependencies.createRpcClient).to.not.have.been.called();
    expect(dependencies.waitForNodesToHaveTheSameHeight).to.not.have.been.called();
    expect(dependencies.dockerCompose.execCommand).to.not.have.been.called();
  });
});
