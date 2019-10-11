const path = require('path');
const dotenvSafe = require('dotenv-safe');
const dotenvExpand = require('dotenv-expand');
const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');

const DriveApiOptions = require('@dashevo/dp-services-ctl/lib/services/drive/api/DriveApiOptions');
const DriveUpdateStateOptions = require('@dashevo/dp-services-ctl/lib/services/drive/updateState/DriveUpdateStateOptions');

const DapiCoreOptions = require('@dashevo/dp-services-ctl/lib/services/dapi/core/DapiCoreOptions');
const DapiTxFilterStreamOptions = require('@dashevo/dp-services-ctl/lib/services/dapi/txFilterStream/DapiTxFilterStreamOptions');
const DashCoreOptions = require('@dashevo/dp-services-ctl/lib/services/dashCore/DashCoreOptions');
const InsightApiOptions = require('@dashevo/dp-services-ctl/lib/services/insightApi/InsightApiOptions');

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

DriveApiOptions.setDefaultCustomOptions(driveOptions);
DriveUpdateStateOptions.setDefaultCustomOptions(driveOptions);

if (process.env.SERVICE_IMAGE_CORE) {
  DashCoreOptions.setDefaultCustomOptions({
    container: {
      image: process.env.SERVICE_IMAGE_CORE,
    },
  });
}

if (process.env.SERVICE_IMAGE_DAPI) {
  DapiCoreOptions.setDefaultCustomOptions({
    container: {
      image: process.env.SERVICE_IMAGE_DAPI,
    },
  });

  DapiTxFilterStreamOptions.setDefaultCustomOptions({
    container: {
      image: process.env.SERVICE_IMAGE_DAPI,
    },
  });
}

if (process.env.SERVICE_IMAGE_INSIGHT) {
  InsightApiOptions.setDefaultCustomOptions({
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
