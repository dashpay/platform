import DockerCompose from '../../../src/docker/DockerCompose.js';
import getConfigMock from '../../../src/test/mock/getConfigMock.js';

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
