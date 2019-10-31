const dotenv = require('dotenv');
const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');

const DriveApiOptions = require('@dashevo/dp-services-ctl/lib/services/drive/api/DriveApiOptions');
const DriveUpdateStateOptions = require(
  '@dashevo/dp-services-ctl/lib/services/drive/updateState/DriveUpdateStateOptions',
);

const DashCoreOptions = require('@dashevo/dp-services-ctl/lib/services/dashCore/DashCoreOptions');

use(sinonChai);
use(chaiAsPromised);
use(dirtyChai);

dotenv.config();

if (process.env.SERVICE_IMAGE_DRIVE) {
  DriveApiOptions.setDefaultCustomOptions({
    container: {
      image: process.env.SERVICE_IMAGE_DRIVE,
      volumes: [
        `${process.cwd()}:/node_modules/@dashevo/dpp:ro`,
        `${process.cwd()}:/usr/src/app/node_modules/@dashevo/dpp:ro`,
      ],
    },
  });

  DriveUpdateStateOptions.setDefaultCustomOptions({
    container: {
      image: process.env.SERVICE_IMAGE_DRIVE,
      volumes: [
        `${process.cwd()}:/node_modules/@dashevo/dpp:ro`,
        `${process.cwd()}:/usr/src/app/node_modules/@dashevo/dpp:ro`,
      ],
    },
  });
}

if (process.env.SERVICE_IMAGE_CORE) {
  DashCoreOptions.setDefaultCustomOptions({
    container: {
      image: process.env.SERVICE_IMAGE_CORE,
    },
  });
}

beforeEach(function beforeEach() {
  if (!this.sinonSandbox) {
    this.sinonSandbox = sinon.createSandbox();
  } else {
    this.sinonSandbox.restore();
  }
});

afterEach(function afterEach() {
  this.sinonSandbox.restore();
});

global.expect = expect;
