import sinon from 'sinon';
import compose from '@dashevo/docker-compose';
import DockerCompose from '../../../src/docker/DockerCompose.js';
import getConfigMock from '../../../src/test/mock/getConfigMock.js';

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

describe('DockerCompose', () => {
  describe('#pullMissingImages', () => {
    let config;
    let docker;
    let getServiceList;
    let dockerPull;
    let dockerCompose;
    let presentImages;

    /**
     * @param {string} image
     * @return {{inspect: function}}
     */
    function getImage(image) {
      return {
        inspect: async () => {
          if (presentImages.includes(image)) {
            return { Id: image };
          }

          const error = new Error(`No such image: ${image}`);
          error.statusCode = 404;

          throw error;
        },
      };
    }

    beforeEach(function it() {
      this.sinon.stub(DockerCompose.prototype, 'throwErrorIfNotInstalled').resolves();

      config = getConfigMock(this.sinon);

      presentImages = [];

      docker = { getImage: this.sinon.stub().callsFake(getImage) };
      getServiceList = this.sinon.stub();

      // A successful pull leaves the image on the host
      dockerPull = this.sinon.stub().callsFake(async (image) => {
        presentImages.push(image);
      });

      dockerCompose = new DockerCompose(
        docker,
        undefined,
        undefined,
        undefined,
        getServiceList,
        dockerPull,
      );
    });

    it('should not pull anything when all images are already on the host', async () => {
      getServiceList.returns([
        {
          name: 'core', image: 'dashpay/dashd:23', isBuiltLocally: false, profiles: ['core'],
        },
        {
          name: 'drive_abci', image: 'dashpay/drive:4', isBuiltLocally: false, profiles: ['platform'],
        },
      ]);

      presentImages = ['dashpay/dashd:23', 'dashpay/drive:4'];

      const pulledImages = await dockerCompose.pullMissingImages(config);

      expect(pulledImages).to.deep.equal([]);
      expect(dockerPull).to.have.not.been.called();
    });

    it('should pull an image that is missing on the host', async () => {
      getServiceList.returns([
        {
          name: 'core', image: 'dashpay/dashd:23', isBuiltLocally: false, profiles: ['core'],
        },
        {
          name: 'drive_abci', image: 'dashpay/drive:4', isBuiltLocally: false, profiles: ['platform'],
        },
      ]);

      presentImages = ['dashpay/dashd:23'];

      const pulledImages = await dockerCompose.pullMissingImages(config);

      expect(pulledImages).to.deep.equal(['dashpay/drive:4']);
      expect(dockerPull).to.have.been.calledOnce();
      expect(dockerPull.firstCall.firstArg).to.equal('dashpay/drive:4');
    });

    it('should report the reason when a missing image can not be pulled', async () => {
      getServiceList.returns([
        {
          name: 'drive_abci', image: 'dashpay/drive:4', isBuiltLocally: false, profiles: ['platform'],
        },
      ]);

      dockerPull.rejects(new Error('write /var/lib/docker: no space left on device'));

      await expect(dockerCompose.pullMissingImages(config))
        .to.be.rejectedWith(/dashpay\/drive:4.*no space left on device/);
    });

    it('should fail when the image is still missing after Docker reported a successful pull', async () => {
      getServiceList.returns([
        {
          name: 'drive_abci', image: 'dashpay/drive:4', isBuiltLocally: false, profiles: ['platform'],
        },
      ]);

      dockerPull.resolves([]);

      await expect(dockerCompose.pullMissingImages(config))
        .to.be.rejectedWith('Failed to pull image dashpay/drive:4: it is still not present on the host');
    });

    it('should report which image could not be checked when Docker fails', async () => {
      getServiceList.returns([
        {
          name: 'drive_abci', image: 'dashpay/drive:4', isBuiltLocally: false, profiles: ['platform'],
        },
      ]);

      docker.getImage.returns({
        inspect: async () => {
          const error = new Error('server error');
          error.statusCode = 500;

          throw error;
        },
      });

      await expect(dockerCompose.pullMissingImages(config))
        .to.be.rejectedWith('Failed to check image dashpay/drive:4: server error');

      expect(dockerPull).to.have.not.been.called();
    });

    it('should stop at the first image that can not be pulled', async () => {
      getServiceList.returns([
        {
          name: 'core', image: 'dashpay/dashd:23', isBuiltLocally: false, profiles: ['core'],
        },
        {
          name: 'drive_abci', image: 'dashpay/drive:4', isBuiltLocally: false, profiles: ['platform'],
        },
        {
          name: 'gateway', image: 'dashpay/envoy:1.39.0', isBuiltLocally: false, profiles: ['platform'],
        },
      ]);

      dockerPull.withArgs('dashpay/drive:4')
        .rejects(new Error('toomanyrequests: You have reached your pull rate limit'));

      await expect(dockerCompose.pullMissingImages(config))
        .to.be.rejectedWith(/dashpay\/drive:4.*toomanyrequests/);

      expect(dockerPull.args.map(([image]) => image))
        .to.deep.equal(['dashpay/dashd:23', 'dashpay/drive:4']);
    });

    it('should not try to pull images built from local sources', async () => {
      getServiceList.returns([
        {
          name: 'drive_abci', image: 'drive:local', isBuiltLocally: true, profiles: ['platform'],
        },
      ]);

      const pulledImages = await dockerCompose.pullMissingImages(config);

      expect(pulledImages).to.deep.equal([]);
      expect(dockerPull).to.have.not.been.called();
    });

    it('should not pull images of services the requested profiles exclude', async () => {
      getServiceList.returns([
        {
          name: 'core', image: 'dashpay/dashd:23', isBuiltLocally: false, profiles: ['core'],
        },
        {
          name: 'insight', image: 'dashpay/insight-api:latest', isBuiltLocally: false, profiles: ['core'],
        },
        {
          name: 'drive_abci', image: 'dashpay/drive:4', isBuiltLocally: false, profiles: ['platform'],
        },
        // Compose always creates a service that declares no profiles
        {
          name: 'dashmate_helper', image: 'dashpay/dashmate-helper:4.1.0', isBuiltLocally: false, profiles: [],
        },
      ]);

      const pulledImages = await dockerCompose.pullMissingImages(config, { profiles: ['platform'] });

      expect(pulledImages).to.deep.equal(['dashpay/drive:4', 'dashpay/dashmate-helper:4.1.0']);
      expect(docker.getImage).to.have.not.been.calledWith('dashpay/insight-api:latest');
    });

    it('should report pull progress', async () => {
      getServiceList.returns([
        {
          name: 'drive_abci', image: 'dashpay/drive:4', isBuiltLocally: false, profiles: ['platform'],
        },
      ]);

      dockerPull.callsFake(async (image, onProgress) => {
        onProgress({ status: 'Downloading', progress: '[====>  ]  12MB/45MB' });
        onProgress({ progressDetail: {} });

        presentImages.push(image);
      });

      const messages = [];

      await dockerCompose.pullMissingImages(config, {
        onProgress: (message) => messages.push(message),
      });

      expect(messages).to.deep.equal(['dashpay/drive:4: Downloading [====>  ]  12MB/45MB']);
    });
  });
});
