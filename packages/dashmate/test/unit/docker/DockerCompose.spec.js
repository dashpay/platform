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

describe('DockerCompose service selection', () => {
  let sandbox;
  let docker;
  let dockerCompose;
  let dockerVolumeLabels;

  beforeEach(() => {
    sandbox = sinon.createSandbox();
    dockerVolumeLabels = {};
    docker = {
      listContainers: sandbox.stub().resolves([]),
      getVolume: sandbox.stub().callsFake((name) => ({
        inspect: async () => {
          if (!Object.hasOwn(dockerVolumeLabels, name)) {
            throw Object.assign(new Error(`no such volume: ${name}`), { statusCode: 404 });
          }

          return { Name: name, Labels: dockerVolumeLabels[name] };
        },
      })),
    };
    dockerCompose = new DockerCompose(docker, null, null, () => ({
      COMPOSE_PROJECT_NAME: 'dashmate_project_node',
    }), () => []);
    sandbox.stub(dockerCompose, 'throwErrorIfNotInstalled').resolves();
  });

  afterEach(() => sandbox.restore());

  describe('volume lookup', () => {
    /**
     * The volumes a reset of the given services removes: the candidates that
     * Docker, asked just before, labels as the project's
     *
     * @param {Object} options
     * @return {Promise<string[]>}
     */
    async function getOwnedVolumeNames(options) {
      const candidates = await dockerCompose.getVolumeCandidates({}, options);
      const isOwned = await Promise.all(
        candidates.map((candidate) => dockerCompose.isVolumeOwned(candidate)),
      );

      return candidates.filter((_, index) => isOwned[index]).map(({ name }) => name);
    }

    /**
     * Docker's labels on a volume Compose created for a project
     *
     * @param {string} project
     * @param {string} volume - the volume's key in the Compose file
     * @return {Object}
     */
    function composeLabels(project, volume) {
      return {
        'com.docker.compose.project': project,
        'com.docker.compose.volume': volume,
      };
    }

    // Compose loads the services the named ones depend on, Core for the
    // quorum list, and their volumes with them.
    it('should return only the named services\' own volumes', async () => {
      dockerVolumeLabels.dashmate_project_node_core_data = composeLabels('dashmate_project_node', 'core_data');
      dockerVolumeLabels.dashmate_project_node_drive_tenderdash = composeLabels('dashmate_project_node', 'drive_tenderdash');

      sandbox.stub(compose, 'execCompose').resolves({
        exitCode: 0,
        err: '',
        out: JSON.stringify({
          name: 'dashmate_project_node',
          services: {
            core: { volumes: [{ type: 'volume', source: 'core_data', target: '/home/dash' }] },
            drive_tenderdash: {
              volumes: [
                { type: 'volume', source: 'drive_tenderdash', target: '/tenderdash' },
                { type: 'bind', source: '/home/user/.dashmate/node/platform', target: '/tenderdash/config' },
              ],
            },
            quorum_list: {},
          },
          volumes: {
            core_data: { name: 'dashmate_project_node_core_data' },
            drive_tenderdash: { name: 'dashmate_project_node_drive_tenderdash' },
          },
        }),
      });

      const volumeNames = await getOwnedVolumeNames({
        serviceNames: ['drive_tenderdash', 'quorum_list'],
        profiles: ['core', 'platform'],
      });

      expect(volumeNames).to.deep.equal(['dashmate_project_node_drive_tenderdash']);
      expect(compose.execCompose.firstCall.args[2].composeOptions)
        .to.deep.equal(['--profile', 'core', '--profile', 'platform']);
      expect(compose.execCompose).to.have.been.calledOnce();
      expect(compose.execCompose.firstCall.args[0]).to.equal('config');
      // Given names, Compose describes only them and what they depend on, and
      // a service that mounts the same volume would go unseen.
      expect(compose.execCompose.firstCall.args[1]).to.deep.equal([]);
    });

    /**
     * @param {Object[]} mounts - the drive_abci service's volumes
     * @param {Object} volumes - the project's volume definitions
     */
    function describeDriveAbci(mounts, volumes) {
      sandbox.stub(compose, 'execCompose').resolves({
        exitCode: 0,
        err: '',
        out: JSON.stringify({
          name: 'dashmate_project_node',
          services: { drive_abci: { volumes: mounts } },
          volumes,
        }),
      });
    }

    // An override can mount a volume this project did not create into a
    // Platform service. Being mounted there does not make it Platform data.
    it('should never return an external volume', async () => {
      dockerVolumeLabels.operator_logs = composeLabels('dashmate_project_node', 'operator_logs');

      describeDriveAbci(
        [{ type: 'volume', source: 'operator_logs', target: '/var/log/operator' }],
        { operator_logs: { name: 'operator_logs', external: true } },
      );

      expect(await getOwnedVolumeNames({ serviceNames: ['drive_abci'] }))
        .to.deep.equal([]);
    });

    // A volume given an explicit name can be one another project created
    // and uses. Only Docker's labels tell whose it is.
    it('should return only volumes Compose created for this project and mount', async () => {
      dockerVolumeLabels.shared_cache = composeLabels('other_project', 'shared_cache');
      dockerVolumeLabels.renamed_data = composeLabels('dashmate_project_node', 'other_key');
      dockerVolumeLabels.dashmate_project_node_drive_abci_data = composeLabels('dashmate_project_node', 'drive_abci_data');

      describeDriveAbci(
        [
          { type: 'volume', source: 'shared_cache', target: '/cache' },
          { type: 'volume', source: 'renamed_data', target: '/renamed' },
          { type: 'volume', source: 'drive_abci_data', target: '/data' },
          { type: 'volume', source: 'unlabeled', target: '/unlabeled' },
        ],
        {
          shared_cache: { name: 'shared_cache' },
          renamed_data: { name: 'renamed_data' },
          drive_abci_data: { name: 'dashmate_project_node_drive_abci_data' },
          unlabeled: { name: 'unlabeled' },
        },
      );
      dockerVolumeLabels.unlabeled = {};

      expect(await getOwnedVolumeNames({ serviceNames: ['drive_abci'] }))
        .to.deep.equal(['dashmate_project_node_drive_abci_data']);
    });

    // Docker gives no labels at all for a volume nothing labelled.
    it('should leave out a volume with no labels', async () => {
      dockerVolumeLabels.dashmate_project_node_drive_abci_data = null;

      describeDriveAbci(
        [{ type: 'volume', source: 'drive_abci_data', target: '/data' }],
        { drive_abci_data: { name: 'dashmate_project_node_drive_abci_data' } },
      );

      expect(await getOwnedVolumeNames({ serviceNames: ['drive_abci'] }))
        .to.deep.equal([]);
    });

    // Two keys can name the same Docker volume, and Docker labels it with
    // the key Compose created it for.
    it('should return a volume two keys name when Docker labels it with either', async () => {
      dockerVolumeLabels.dashmate_project_node_drive_abci_data = composeLabels('dashmate_project_node', 'drive_abci_data');

      describeDriveAbci(
        [
          { type: 'volume', source: 'drive_abci_data', target: '/data' },
          { type: 'volume', source: 'drive_abci_alias', target: '/alias' },
        ],
        {
          drive_abci_data: { name: 'dashmate_project_node_drive_abci_data' },
          drive_abci_alias: { name: 'dashmate_project_node_drive_abci_data' },
        },
      );

      expect(await getOwnedVolumeNames({ serviceNames: ['drive_abci'] }))
        .to.deep.equal(['dashmate_project_node_drive_abci_data']);
    });

    /**
     * @param {Object<string, Object[]>} serviceMounts - each service's volumes
     * @param {Object} volumes - the project's volume definitions
     */
    function describeServices(serviceMounts, volumes) {
      sandbox.stub(compose, 'execCompose').resolves({
        exitCode: 0,
        err: '',
        out: JSON.stringify({
          name: 'dashmate_project_node',
          services: Object.fromEntries(Object.entries(serviceMounts)
            .map(([name, mounts]) => [name, { volumes: mounts }])),
          volumes,
        }),
      });
    }

    // Docker knows a volume by its name, and two keys can give the same one.
    // Declaring it external under one key says the operator manages it, even
    // though the other key is a plain one Compose created it for.
    it('should leave out a volume one of its keys declares external', async () => {
      dockerVolumeLabels.dashmate_project_node_drive_abci_data = composeLabels('dashmate_project_node', 'drive_abci_data');

      describeServices(
        {
          drive_abci: [
            { type: 'volume', source: 'drive_abci_data', target: '/data' },
            { type: 'volume', source: 'drive_abci_managed', target: '/managed' },
          ],
        },
        {
          drive_abci_data: { name: 'dashmate_project_node_drive_abci_data' },
          drive_abci_managed: { name: 'dashmate_project_node_drive_abci_data', external: true },
        },
      );

      expect(await getOwnedVolumeNames({ serviceNames: ['drive_abci'] }))
        .to.deep.equal([]);
    });

    // Removing a volume a service outside the reset also mounts would take
    // that service's data, Core's included, once nothing holds it in use.
    it('should leave out a volume a service it does not reset mounts too', async () => {
      dockerVolumeLabels.dashmate_project_node_shared_data = composeLabels('dashmate_project_node', 'shared_data');
      // Compose created it for the Platform key, starting that service first.
      dockerVolumeLabels.dashmate_project_node_shared_cache = composeLabels('dashmate_project_node', 'drive_abci_cache');
      dockerVolumeLabels.dashmate_project_node_drive_abci_data = composeLabels('dashmate_project_node', 'drive_abci_data');

      describeServices(
        {
          core: [
            { type: 'volume', source: 'shared_data', target: '/shared' },
            { type: 'volume', source: 'core_cache', target: '/cache' },
          ],
          drive_abci: [
            { type: 'volume', source: 'shared_data', target: '/shared' },
            { type: 'volume', source: 'drive_abci_cache', target: '/cache' },
            { type: 'volume', source: 'drive_abci_data', target: '/data' },
          ],
        },
        {
          shared_data: { name: 'dashmate_project_node_shared_data' },
          // Two keys for one volume, one for each service
          core_cache: { name: 'dashmate_project_node_shared_cache' },
          drive_abci_cache: { name: 'dashmate_project_node_shared_cache' },
          drive_abci_data: { name: 'dashmate_project_node_drive_abci_data' },
        },
      );

      expect(await getOwnedVolumeNames({ serviceNames: ['drive_abci'] }))
        .to.deep.equal(['dashmate_project_node_drive_abci_data']);
    });

    // Only a missing volume means there is nothing to remove.
    it('should fail when Docker cannot tell whose a volume is', async () => {
      docker.getVolume.callsFake(() => ({
        inspect: () => Promise.reject(Object.assign(new Error('Docker unavailable'), { statusCode: 500 })),
      }));

      describeDriveAbci(
        [{ type: 'volume', source: 'drive_abci_data', target: '/data' }],
        { drive_abci_data: { name: 'dashmate_project_node_drive_abci_data' } },
      );

      await expect(getOwnedVolumeNames({ serviceNames: ['drive_abci'] }))
        .to.be.rejectedWith('Docker unavailable');
    });

    // Nothing to remove for a volume that was never created, unless it is
    // created by the time it would be removed: whether Docker has it is
    // asked then, not when the candidates are listed.
    it('should leave out a volume Docker does not have when asked', async () => {
      describeDriveAbci(
        [{ type: 'volume', source: 'drive_abci_data', target: '/data' }],
        { drive_abci_data: { name: 'dashmate_project_node_drive_abci_data' } },
      );

      const candidates = await dockerCompose.getVolumeCandidates({}, { serviceNames: ['drive_abci'] });

      expect(candidates).to.deep.equal([{
        name: 'dashmate_project_node_drive_abci_data',
        project: 'dashmate_project_node',
        keys: ['drive_abci_data'],
      }]);
      expect(await dockerCompose.isVolumeOwned(candidates[0])).to.be.false();

      dockerVolumeLabels.dashmate_project_node_drive_abci_data = composeLabels('dashmate_project_node', 'drive_abci_data');

      expect(await dockerCompose.isVolumeOwned(candidates[0])).to.be.true();
    });

    // A service missing from the description has volumes nobody would remove.
    it('should fail when Compose does not describe a named service', async () => {
      sandbox.stub(compose, 'execCompose').resolves({
        exitCode: 0,
        err: '',
        out: JSON.stringify({ name: 'dashmate_project_node', services: { core: {} } }),
      });

      await expect(getOwnedVolumeNames({ serviceNames: ['drive_abci'] }))
        .to.be.rejectedWith("did not describe service 'drive_abci'");
    });

    // The lookup can be the reset's first contact with Docker, before any
    // running-services check and always with --force. Docker's own guidance
    // (not installed, not started, no permission) must show there too, not
    // the raw error of the Compose command or the socket.
    it('should check Docker is available before it asks Compose', async () => {
      dockerCompose.throwErrorIfNotInstalled.rejects(new Error("Can't connect to Docker Engine: permission denied"));
      sandbox.stub(compose, 'execCompose').rejects(new Error('unexpected call'));

      await expect(getOwnedVolumeNames({ serviceNames: ['drive_abci'] }))
        .to.be.rejectedWith("Can't connect to Docker Engine");

      expect(compose.execCompose).to.not.have.been.called();
    });

    it('should not ask Compose about an empty selection', async () => {
      sandbox.stub(compose, 'execCompose').rejects(new Error('unexpected call'));

      expect(await dockerCompose.getVolumeCandidates({}, { serviceNames: [] })).to.deep.equal([]);
    });
  });

  describe('isNodeRunning', () => {
    /**
     * @param {Object[]} containers as `docker compose ps --format json` lists them
     */
    function listContainers(containers) {
      sandbox.stub(compose, 'ps').resolves({ data: { json: containers } });
    }

    // `docker compose ps <names>` fails before Compose 2.15 when a named
    // service has no container, and that failure read as "not running".
    // Without names, older versions leave out the services whose profile is
    // not enabled, so every profile given is enabled.
    it('should list the project\'s containers with the given profiles and pick the named services', async () => {
      listContainers([
        { Service: 'core', State: 'running' },
        { Service: 'drive_abci', State: 'exited' },
      ]);

      expect(await dockerCompose.isNodeRunning({}, {
        serviceNames: ['drive_abci', 'quorum_list'],
        profiles: ['core', 'platform'],
      })).to.be.false();

      const [{ composeOptions, commandOptions }] = compose.ps.firstCall.args;
      expect(composeOptions).to.deep.equal(['--profile', 'core', '--profile', 'platform']);
      expect(commandOptions).to.deep.equal(['--format', 'json']);

      compose.ps.resolves({ data: { json: [{ Service: 'drive_abci', State: 'running' }] } });

      expect(await dockerCompose.isNodeRunning({}, { serviceNames: ['drive_abci'] })).to.be.true();
    });

    // Stop, reindex and the full reset ask about the services the config
    // enables. Passed to Compose by name, one with no container (a sidecar
    // never started) made Compose before 2.15 fail, which read as nothing
    // running while Core was up, and each caller would have to know that.
    it('should find a running Core next to an enabled service with no container', async () => {
      const nodeCompose = new DockerCompose(docker, null, null, () => ({
        COMPOSE_PROJECT_NAME: 'dashmate_project_node',
      }), () => [
        { name: 'core', profiles: ['core'] },
        { name: 'core_tor', profiles: ['core'] },
        { name: 'drive_abci', profiles: ['platform'] },
      ]);
      sandbox.stub(nodeCompose, 'throwErrorIfNotInstalled').resolves();

      sandbox.stub(compose, 'ps').callsFake(async ({ commandOptions }) => {
        const namedServices = commandOptions.flat().filter((option) => ['core', 'core_tor', 'drive_abci'].includes(option));
        if (namedServices.includes('core_tor')) {
          // eslint-disable-next-line prefer-promise-reject-errors
          return Promise.reject({ exitCode: 1, out: '', err: 'no such service: core_tor\n' });
        }

        return { data: { json: [{ Service: 'core', State: 'running' }] } };
      });

      expect(await nodeCompose.isNodeRunning({})).to.be.true();

      // A profile filter picks services and enables no profile: the config's
      // own make up the model, which without Core can be invalid.
      expect(await nodeCompose.isNodeRunning({}, { profiles: ['platform'] })).to.be.false();
      expect(await nodeCompose.isNodeRunning({}, { profiles: ['core'] })).to.be.true();
      compose.ps.getCalls().forEach(({ args: [{ composeOptions }] }) => {
        expect(composeOptions).to.deep.equal([]);
      });
    });

    it('should report nothing running for an empty selection', async () => {
      listContainers([{ Service: 'core', State: 'running' }]);

      expect(await dockerCompose.isNodeRunning({}, { serviceNames: [] })).to.be.false();
      expect(compose.ps).to.not.have.been.called();
    });
  });
});
