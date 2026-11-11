import DockerImageCleanup from '../../../src/docker/DockerImageCleanup.js';

describe('DockerImageCleanup', () => {
  const now = Date.parse('2026-11-11T12:00:00Z');
  const day = 24 * 60 * 60 * 1000;
  let docker;
  let images;
  let containers;
  let removed;
  let cleanup;
  let configuredReferences;

  function addImage(id, overrides = {}) {
    const image = {
      Id: id,
      Created: new Date(now - 730 * day).toISOString(),
      RepoTags: [`dashpay/drive:${id}`],
      RepoDigests: [],
      ...overrides,
    };
    images.set(id, image);
    image.RepoTags.forEach((reference) => images.set(reference, image));
    image.RepoDigests.forEach((reference) => images.set(reference, image));
    return image;
  }

  beforeEach(function beforeEach() {
    this.sinon.stub(Date, 'now').returns(now);
    images = new Map();
    containers = [];
    removed = [];
    configuredReferences = new Set();
    docker = {
      info: this.sinon.stub().resolves({ ID: 'daemon' }),
      listContainers: this.sinon.stub().callsFake(async () => containers),
      listImages: this.sinon.stub().callsFake(async () => [...new Set(images.values())]),
      getImage: this.sinon.stub().callsFake((reference) => ({
        inspect: async () => {
          if (!images.has(reference)) {
            throw Object.assign(new Error('not found'), { statusCode: 404 });
          }
          return images.get(reference);
        },
        remove: async (options) => {
          expect(options).to.deep.equal({ force: false, noprune: true });
          removed.push(reference);
        },
      })),
    };
    cleanup = new DockerImageCleanup(
      docker,
      () => [{ image: 'dashpay/drive:current' }],
      () => ({ COMPOSE_PROJECT_NAME: 'dashmate_node' }),
      () => configuredReferences,
    );
    addImage('current', { RepoTags: ['dashpay/drive:current'] });
  });

  it('should reclaim two years of tagged historical images on the first update', async () => {
    addImage('drive-2024');
    addImage('legacy-dapi', { RepoTags: ['dashpay/dapi:0.25'] });
    addImage('helper', { RepoTags: ['dashpay/dashmate-helper:1.0.0'] });
    addImage('core', { RepoTags: ['dashpay/dashd:20'] });

    const snapshot = await cleanup.prepare({});
    docker.listContainers.resetHistory();
    const result = await cleanup.cleanup(snapshot);

    expect(result.deleted).to.deep.equal(['drive-2024', 'legacy-dapi', 'helper', 'core']);
    expect(removed).to.deep.equal(result.deleted);
    expect(docker.listImages).to.have.been.calledOnceWithExactly({ all: true });
    expect(docker.listContainers).to.have.been.calledOnceWithExactly({ all: true });
  });

  it('should recognize historical digest-only images and canonical Docker Hub references', async () => {
    addImage('digest', { RepoTags: [], RepoDigests: ['dashpay/drive@sha256:old'] });
    addImage('hub', { RepoTags: ['docker.io/dashpay/tenderdash:0.11'] });
    addImage('index', { RepoTags: ['index.docker.io/dashpay/envoy:old'] });

    await cleanup.cleanup(await cleanup.prepare({}));

    expect(removed).to.deep.equal(['digest', 'hub', 'index']);
  });

  it('should recognize fully dangling historical images by specific build labels', async () => {
    for (const description of ['Drive ABCI Rust', 'DAPI Node.JS', 'Dashmate Helper Node.JS',
      'Dash Platform API (DAPI) - Rust Implementation']) {
      addImage(description, {
        RepoTags: ['<none>:<none>'],
        RepoDigests: ['<none>@<none>'],
        Config: { Labels: { maintainer: 'Dash Developers <dev@dash.org>', description } },
      });
    }

    await cleanup.cleanup(await cleanup.prepare({}));

    expect(removed).to.have.length(4);
  });

  it('should leave unrelated repositories, aliases, registries and ambiguous dangling images alone', async () => {
    for (const reference of ['redis:old', 'dashpay/drive-extra:old', 'example.com/dashpay/drive:old',
      'example.com:5000/dashpay/drive:old', 'drive:local']) {
      addImage(reference, { RepoTags: [reference] });
    }
    addImage('aliased', { RepoTags: ['dashpay/drive:old', 'operator/backup:old'] });
    addImage('foreign-digest', { RepoDigests: ['example.com/dashpay/drive@sha256:old'] });
    addImage('unknown', { RepoTags: [], Config: { Labels: {} } });
    addImage('test-suite', {
      RepoTags: [],
      Config: { Labels: { maintainer: 'Dash Developers <dev@dash.org>', description: 'Dash Platform test suite' } },
    });
    addImage('wrong-maintainer', {
      RepoTags: [],
      Config: { Labels: { maintainer: 'someone else', description: 'Drive ABCI Rust' } },
    });

    await cleanup.cleanup(await cleanup.prepare({}));

    expect(removed).to.deep.equal([]);
  });

  it('should delete only images strictly older than fourteen days', async () => {
    addImage('old', { Created: new Date(now - 14 * day - 1).toISOString() });
    addImage('boundary', { Created: new Date(now - 14 * day).toISOString() });
    addImage('recent', { Created: new Date(now - day).toISOString() });
    addImage('invalid', { Created: 'unknown' });

    await cleanup.cleanup(await cleanup.prepare({}));

    expect(removed).to.deep.equal(['old']);
  });

  it('should retain old-built newly pulled images and the previous mutable-tag image', async () => {
    const snapshot = await cleanup.prepare({});
    images.get('current').RepoTags = [];
    addImage('new-current', { RepoTags: ['dashpay/drive:current'] });

    await cleanup.cleanup(snapshot);

    expect(removed).to.deep.equal([]);
  });

  it('should retain images configured for other or disabled services without containers', async () => {
    addImage('other', { RepoTags: ['dashpay/drive:other'] });
    configuredReferences.add('dashpay/drive:other');
    configuredReferences.add('not-present:latest');

    await cleanup.cleanup(await cleanup.prepare({}));

    expect(removed).to.deep.equal([]);
  });

  it('should retain an image when a configured tag moves during candidate listing', async () => {
    const reference = 'dashpay/drive:other';
    const previous = addImage('previous', { RepoTags: [reference] });
    const next = addImage('next');
    addImage('unused');
    configuredReferences.add(reference);
    docker.listImages.callsFake(async () => {
      previous.RepoTags = [];
      next.RepoTags.push(reference);
      images.set(reference, next);
      return [...new Set(images.values())];
    });

    await cleanup.cleanup(await cleanup.prepare({}));

    expect(removed).to.deep.equal(['unused']);
  });

  it('should refresh configured image IDs again after an earlier candidate is removed', async () => {
    const reference = 'dashpay/drive:other';
    const previous = addImage('previous', { RepoTags: [reference] });
    const first = addImage('first');
    const next = addImage('next');
    configuredReferences.add(reference);
    docker.getImage.withArgs('first').returns({
      inspect: async () => first,
      remove: async (options) => {
        expect(options).to.deep.equal({ force: false, noprune: true });
        removed.push(first.Id);
        previous.RepoTags = [];
        next.RepoTags.push(reference);
        images.set(reference, next);
      },
    });

    await cleanup.cleanup(await cleanup.prepare({}));

    expect(removed).to.deep.equal(['first']);
  });

  it('should retain all running and stopped container images, including unrelated containers', async () => {
    addImage('running');
    addImage('stopped');
    containers = [
      { ImageID: 'running', State: 'running', Labels: {} },
      { ImageID: 'stopped', State: 'exited', Labels: {} },
    ];

    const result = await cleanup.cleanup(await cleanup.prepare({}));

    expect(removed).to.deep.equal([]);
    expect(result.skipped).to.deep.equal(['running', 'stopped']);
    expect(docker.listContainers).to.have.always.been.calledWithExactly({ all: true });
  });

  it('should retain the previous node images even if its containers disappear during the update', async () => {
    addImage('old-node');
    containers = [{
      ImageID: 'old-node',
      Labels: { 'com.docker.compose.project': 'dashmate_node', 'org.dashmate.service.title': 'Drive' },
    }];
    const snapshot = await cleanup.prepare({});
    containers = [];

    await cleanup.cleanup(snapshot);

    expect(removed).to.deep.equal([]);
  });

  it('should skip images that become referenced after preparing the update', async () => {
    addImage('new-container');
    const snapshot = await cleanup.prepare({});
    containers.push({ ImageID: 'new-container' });

    await cleanup.cleanup(snapshot);

    expect(removed).to.deep.equal([]);
  });

  it('should let Docker protect a container created after the container snapshot', async function it() {
    const image = addImage('new-container');
    const remove = this.sinon.stub()
      .rejects(Object.assign(new Error('image is in use'), { statusCode: 409 }));
    docker.listImages.callsFake(async () => {
      containers.push({ ImageID: image.Id });
      return [image];
    });
    docker.getImage.withArgs(image.Id).returns({
      inspect: async () => image,
      remove,
    });
    const snapshot = await cleanup.prepare({});
    docker.listContainers.resetHistory();

    const result = await cleanup.cleanup(snapshot);

    expect(result.deleted).to.deep.equal([]);
    expect(result.skipped).to.deep.equal([image.Id]);
    expect(docker.listContainers).to.have.been.calledOnceWithExactly({ all: true });
    expect(remove).to.have.been.calledOnceWithExactly({ force: false, noprune: true });
  });

  it('should re-inspect aliases after listing images', async () => {
    const image = addImage('retagged');
    docker.listImages.callsFake(async () => {
      image.RepoTags.push('operator/backup:old');
      return [{ Id: image.Id }];
    });

    await cleanup.cleanup(await cleanup.prepare({}));

    expect(removed).to.deep.equal([]);
  });

  [404, 409].forEach((statusCode) => {
    it(`should tolerate concurrent image removal/conflict (${statusCode})`, async function it() {
      addImage('raced');
      docker.getImage.withArgs('raced').returns({
        inspect: async () => images.get('raced'),
        remove: this.sinon.stub().rejects(Object.assign(new Error('race'), { statusCode })),
      });
      addImage('another');

      const result = await cleanup.cleanup(await cleanup.prepare({}));

      expect(result.deleted).to.deep.equal(['another']);
      expect(result.skipped).to.deep.equal(statusCode === 409 ? ['raced'] : []);
    });
  });

  it('should abort without deletion when the daemon changes', async () => {
    addImage('old');
    const snapshot = await cleanup.prepare({});
    docker.info.resolves({ ID: 'different-daemon' });

    await expect(cleanup.cleanup(snapshot)).to.be.rejectedWith('Docker daemon changed');
    expect(removed).to.deep.equal([]);
  });

  it('should abort without deletion when protection references cannot be resolved', async function it() {
    addImage('old');
    configuredReferences.add('denied:latest');
    docker.getImage.withArgs('denied:latest').returns({
      inspect: this.sinon.stub().rejects(new Error('permission denied')),
    });

    await expect(cleanup.cleanup(await cleanup.prepare({}))).to.be.rejectedWith('permission denied');
    expect(removed).to.deep.equal([]);
  });

  it('should abort without deletion if a successfully pulled image disappeared', async () => {
    addImage('old');
    const snapshot = await cleanup.prepare({});
    images.delete('dashpay/drive:current');

    await expect(cleanup.cleanup(snapshot)).to.be.rejectedWith('no longer available');
    expect(removed).to.deep.equal([]);
  });
});
