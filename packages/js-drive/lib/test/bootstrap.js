const path = require('path');
const dotenvSafe = require('dotenv-safe');
const dotenvExpand = require('dotenv-expand');
const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');
const DashApiOptions = require('@dashevo/js-evo-services-ctl/lib/services/driveApi/DriveApiOptions');
const DashSyncOptions = require('@dashevo/js-evo-services-ctl/lib/services/driveSync/DriveSyncOptions');
const DapiOptions = require('@dashevo/js-evo-services-ctl/lib/services/dapi/DapiOptions');
const DashCoreOptions = require('@dashevo/js-evo-services-ctl/lib/services/dashCore/DashCoreOptions');
const InsightOptions = require('@dashevo/js-evo-services-ctl/lib/services/insight/InsightOptions');

use(sinonChai);
use(chaiAsPromised);
use(dirtyChai);

process.env.NODE_ENV = 'test';

const dotenvConfig = dotenvSafe.config({
  path: path.resolve(__dirname, '..', '..', '.env'),
});
dotenvExpand(dotenvConfig);

const rootPath = process.cwd();

const driveContainerOptions = {
  throwErrorsFromLog: true,
  volumes: [
    `${rootPath}/lib:/usr/src/app/lib`,
    `${rootPath}/scripts:/usr/src/app/scripts`,
    `${rootPath}/.env:/usr/src/app/.env`,
    `${rootPath}/.env.example:/usr/src/app/.env.example`,
  ],
};

if (process.env.SERVICE_IMAGE_DRIVE) {
  Object.assign(driveContainerOptions, {
    image: process.env.SERVICE_IMAGE_DRIVE,
  });
}

const driveOptions = {
  cacheNodeModules: true,
  localAppPath: rootPath,
  container: driveContainerOptions,
};

DashApiOptions.setDefaultCustomOptions(driveOptions);
DashSyncOptions.setDefaultCustomOptions(driveOptions);

if (process.env.SERVICE_IMAGE_CORE) {
  DashCoreOptions.setDefaultCustomOptions({
    container: {
      image: process.env.SERVICE_IMAGE_CORE,
    },
  });
}

if (process.env.SERVICE_IMAGE_DAPI) {
  DapiOptions.setDefaultCustomOptions({
    container: {
      image: process.env.SERVICE_IMAGE_DAPI,
    },
  });
}

if (process.env.SERVICE_IMAGE_INSIGHT) {
  InsightOptions.setDefaultCustomOptions({
    container: {
      image: process.env.SERVICE_IMAGE_INSIGHT,
    },
  });
}

beforeEach(function beforeEach() {
  if (!this.sinon) {
    this.sinon = sinon.createSandbox();
  } else {
    this.sinon.restore();
  }
});

afterEach(function afterEach() {
  this.sinon.restore();
});

global.expect = expect;
