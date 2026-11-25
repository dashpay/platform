import selectGroupBuildConfigs from '../../../src/util/selectGroupBuildConfigs.js';

/**
 * A config that answers paths the way Config does, subtrees included
 *
 * @param {string} name
 * @param {Object} options
 * @return {{get: function(string): *, getName: function(): string}}
 */
function createConfig(name, options) {
  return {
    getName: () => name,
    get: (path) => path.split('.').reduce((value, key) => value?.[key], options),
  };
}

/**
 * @param {Object} [overrides]
 * @param {boolean} [overrides.drive]
 * @param {boolean} [overrides.rsDapi]
 * @param {boolean} [overrides.baseImage]
 * @param {string} [overrides.driveImage]
 * @param {string} [overrides.driveTarget]
 * @param {string} [overrides.baseTarget]
 * @return {Object}
 */
function nodeOptions({
  drive = false,
  rsDapi = false,
  baseImage = false,
  driveImage = 'dashpay/drive:5',
  driveTarget = 'drive-abci',
  baseTarget = '',
} = {}) {
  const build = (enabled, target) => ({
    enabled, context: '/src/platform', dockerFile: '/src/platform/Dockerfile', target, buildArgs: {},
  });

  return {
    docker: { baseImage: { build: build(baseImage, baseTarget) } },
    dashmate: { helper: { docker: { image: 'dashpay/dashmate-helper:5', build: build(false, 'dashmate-helper') } } },
    platform: {
      enable: true,
      drive: { abci: { docker: { image: driveImage, build: build(drive, driveTarget) } } },
      dapi: { rsDapi: { docker: { image: 'dashpay/rs-dapi:5', build: build(rsDapi, 'rs-dapi') } } },
    },
  };
}

describe('selectGroupBuildConfigs', () => {
  it('should build once for nodes that build the same images', () => {
    const configs = ['local_1', 'local_2', 'local_3'].map((name) => (
      createConfig(name, nodeOptions({ drive: true, rsDapi: true }))
    ));

    expect(selectGroupBuildConfigs(configs)).to.deep.equal([configs[0]]);
  });

  it('should build for each node that builds images no other node builds', () => {
    const drive = createConfig('local_1', nodeOptions({ drive: true }));
    const rsDapi = createConfig('local_2', nodeOptions({ rsDapi: true }));

    expect(selectGroupBuildConfigs([drive, rsDapi])).to.deep.equal([drive, rsDapi]);
  });

  it('should not build for a node whose images another node already builds', () => {
    const both = createConfig('local_1', nodeOptions({ drive: true, rsDapi: true }));
    const rsDapi = createConfig('local_2', nodeOptions({ rsDapi: true }));

    expect(selectGroupBuildConfigs([both, rsDapi])).to.deep.equal([both]);
  });

  // The build files tag their output `drive:local`, `rs-dapi:local` and so on,
  // whatever image a node is configured with; that image only seeds the build cache
  it('should build once for nodes that build the same service under different configured images', () => {
    const first = createConfig('local_1', nodeOptions({ drive: true }));
    const second = createConfig('local_2', nodeOptions({ drive: true, driveImage: 'dashpay/drive:other' }));

    expect(selectGroupBuildConfigs([first, second])).to.deep.equal([first]);
  });

  // A group on one host shares one local image per service, so two different
  // builds of it would leave every node running whichever was built last
  it('should refuse nodes that build the same service differently', () => {
    const first = createConfig('local_1', nodeOptions({ drive: true }));
    const second = createConfig('local_2', nodeOptions({ drive: true, driveTarget: 'drive-abci-debug' }));

    expect(() => selectGroupBuildConfigs([first, second]))
      .to.throw(/local_1 and local_2 build the drive image differently/);
  });

  // Configs are written by different commands, so the same build arguments can
  // be stored in a different order
  it('should build once for nodes whose build settings differ only in key order', () => {
    const firstOptions = nodeOptions({ drive: true });
    const secondOptions = nodeOptions({ drive: true });

    firstOptions.platform.drive.abci.docker.build.buildArgs = { SDK_TEST_DATA: 'true', CARGO_BUILD_PROFILE: 'dev' };
    secondOptions.platform.drive.abci.docker.build.buildArgs = { CARGO_BUILD_PROFILE: 'dev', SDK_TEST_DATA: 'true' };

    const first = createConfig('local_1', firstOptions);
    const second = createConfig('local_2', secondOptions);

    expect(selectGroupBuildConfigs([first, second])).to.deep.equal([first]);
  });

  it('should refuse nodes that build the base image differently', () => {
    const first = createConfig('local_1', nodeOptions({ drive: true, baseImage: true }));
    const second = createConfig('local_2', nodeOptions({ drive: true, baseImage: true, baseTarget: 'other' }));

    expect(() => selectGroupBuildConfigs([first, second]))
      .to.throw(/local_1 and local_2 build the base image differently/);
  });

  it('should not build for nodes that use released images', () => {
    const configs = [createConfig('local_1', nodeOptions()), createConfig('local_2', nodeOptions())];

    expect(selectGroupBuildConfigs(configs)).to.deep.equal([]);
  });
});
