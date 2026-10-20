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
 * @param {string} [overrides.driveImage]
 * @return {Object}
 */
function nodeOptions({ drive = false, rsDapi = false, driveImage = 'drive:local' } = {}) {
  const build = (enabled, target) => ({
    enabled, context: '/src/platform', dockerFile: '/src/platform/Dockerfile', target, buildArgs: {},
  });

  return {
    docker: { baseImage: { build: { enabled: false, context: '/src/platform', target: '' } } },
    dashmate: { helper: { docker: { image: 'dashmate-helper:local', build: build(false, 'dashmate-helper') } } },
    platform: {
      enable: true,
      drive: { abci: { docker: { image: driveImage, build: build(drive, 'drive-abci') } } },
      dapi: { rsDapi: { docker: { image: 'rs-dapi:local', build: build(rsDapi, 'rs-dapi') } } },
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

  it('should build for each node that builds different services', () => {
    const drive = createConfig('local_1', nodeOptions({ drive: true }));
    const rsDapi = createConfig('local_2', nodeOptions({ rsDapi: true }));

    expect(selectGroupBuildConfigs([drive, rsDapi])).to.deep.equal([drive, rsDapi]);
  });

  // The same service built under another tag is another image
  it('should build for each node that tags the same service differently', () => {
    const first = createConfig('local_1', nodeOptions({ drive: true }));
    const second = createConfig('local_2', nodeOptions({ drive: true, driveImage: 'drive:other' }));

    expect(selectGroupBuildConfigs([first, second])).to.deep.equal([first, second]);
  });

  it('should not build for nodes that use released images', () => {
    const configs = [createConfig('local_1', nodeOptions()), createConfig('local_2', nodeOptions())];

    expect(selectGroupBuildConfigs(configs)).to.deep.equal([]);
  });
});
