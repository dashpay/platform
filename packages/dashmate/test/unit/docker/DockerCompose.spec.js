import sinon from 'sinon';
import compose from '@dashevo/docker-compose';
import DockerCompose from '../../../src/docker/DockerCompose.js';

describe('DockerCompose Tor lifecycle', () => {
  let sandbox;
  let docker;
  let sidecar;
  let config;
  let dockerCompose;

  beforeEach(() => {
    sandbox = sinon.createSandbox();
    sidecar = {
      stop: sandbox.stub().resolves(),
      remove: sandbox.stub().resolves(),
    };
    docker = {
      listContainers: sandbox.stub().resolves([{ Id: 'tor-container' }]),
      getContainer: sandbox.stub().withArgs('tor-container').returns(sidecar),
    };
    config = { get: sandbox.stub().withArgs('core.tor.enabled').returns(false) };
    dockerCompose = new DockerCompose(docker, null, null, () => ({
      COMPOSE_PROJECT_NAME: 'dashmate_project_node',
    }));
    sandbox.stub(dockerCompose, 'throwErrorIfNotInstalled').resolves();
    sandbox.stub(compose, 'upAll').resolves();
    sandbox.stub(compose, 'stop').resolves();
  });

  afterEach(() => sandbox.restore());

  ['up', 'stop'].forEach((operation) => {
    describe(operation, () => {
      it('should remove a disabled sidecar before operating on Core and retain its volumes', async () => {
        await dockerCompose[operation](config);

        expect(docker.listContainers).to.have.been.calledOnceWithExactly({
          all: true,
          filters: {
            label: [
              'com.docker.compose.project=dashmate_project_node',
              'com.docker.compose.service=core_tor',
            ],
          },
        });
        expect(sidecar.stop).to.have.been.calledOnceWithExactly({ t: 30 });
        expect(sidecar.remove).to.have.been.calledOnceWithExactly();
        sinon.assert.callOrder(sidecar.stop, sidecar.remove, compose[operation === 'up' ? 'upAll' : 'stop']);
      });

      it('should leave an enabled sidecar alone', async () => {
        config.get.withArgs('core.tor.enabled').returns(true);

        await dockerCompose[operation](config);

        expect(docker.listContainers).to.not.have.been.called();
      });

      it('should leave Core services alone during Platform-only operations', async () => {
        await dockerCompose[operation](config, { profiles: ['platform', 'platform-dapi-rs'] });

        expect(docker.listContainers).to.not.have.been.called();
      });

      it('should clean up when the core profile is explicitly selected', async () => {
        await dockerCompose[operation](config, { profiles: ['core'] });

        expect(sidecar.remove).to.have.been.calledOnce();
      });

      it('should succeed when no sidecar exists', async () => {
        docker.listContainers.resolves([]);

        await dockerCompose[operation](config);

        expect(docker.getContainer).to.not.have.been.called();
      });

      it('should remove a sidecar that was already stopped', async () => {
        sidecar.stop.rejects(Object.assign(new Error('already stopped'), { statusCode: 304 }));

        await dockerCompose[operation](config);

        expect(sidecar.remove).to.have.been.calledOnce();
      });

      ['stop', 'remove'].forEach((step) => {
        it(`should tolerate a sidecar disappearing during ${step}`, async () => {
          sidecar[step].rejects(Object.assign(new Error('not found'), { statusCode: 404 }));

          await dockerCompose[operation](config);
        });

        it(`should report ${step} failures instead of silently leaving Tor running`, async () => {
          sidecar[step].rejects(new Error('Docker unavailable'));

          await expect(dockerCompose[operation](config)).to.be.rejectedWith('Docker unavailable');

          expect(compose.upAll).to.not.have.been.called();
          expect(compose.stop).to.not.have.been.called();
        });
      });
    });
  });
});
