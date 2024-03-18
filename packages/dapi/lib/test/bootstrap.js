const path = require('path');
const dotenvSafe = require('dotenv-safe');
const dotenvExpand = require('dotenv-expand');
const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');

const DashCoreOptions = require('@dashevo/dp-services-ctl/lib/services/dashCore/DashCoreOptions');

use(sinonChai);
use(chaiAsPromised);
use(dirtyChai);

process.env.NODE_ENV = 'test';

const dotenvConfig = dotenvSafe.config({
  path: path.resolve(__dirname, '..', '..', '.env'),
});
dotenvExpand(dotenvConfig);

process.env.LOG_LEVEL = 'silent';

const rootPath = process.cwd();

const dapiContainerOptions = {
  volumes: [
    `${rootPath}/lib:/platform/packages/dapi/lib`,
    `${rootPath}/scripts:/platform/packages/dapi/scripts`,
  ],
};

const dapiOptions = {
  cacheNodeModules: true,
  localAppPath: rootPath,
  container: dapiContainerOptions,
};

if (process.env.SERVICE_IMAGE_DAPI) {
  dapiOptions.container = {
    image: process.env.SERVICE_IMAGE_DAPI,
    ...dapiContainerOptions,
  };
}

if (process.env.SERVICE_IMAGE_CORE) {
  DashCoreOptions.setDefaultCustomOptions({
    container: {
      image: process.env.SERVICE_IMAGE_CORE,
    },
  });
}

exports.mochaHooks = {
  beforeEach() {
    if (!this.sinon) {
      this.sinon = sinon.createSandbox();
    } else {
      this.sinon.restore();
    }
  },

  afterEach() {
    this.sinon.restore();
  },
};

global.expect = expect;
