const path = require('path');
const dotenvSafe = require('dotenv-safe');
const dotenvExpand = require('dotenv-expand');
const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');
const DashDriveOptions = require('js-evo-services-ctl/lib/dashDrive/DashDriveOptions');

use(sinonChai);
use(chaiAsPromised);
use(dirtyChai);

process.env.NODE_ENV = 'test';

const dotenvConfig = dotenvSafe.config({
  path: path.resolve(__dirname, '..', '..', '.env'),
});
dotenvExpand(dotenvConfig);

const rootPath = process.cwd();
const options = {
  container: {
    volumes: [
      `${rootPath}/lib:/usr/src/app/lib`,
      `${rootPath}/scripts:/usr/src/app/scripts`,
      `${rootPath}/package.json:/usr/src/app/package.json`,
      `${rootPath}/package-lock.json:/usr/src/app/package-lock.json`,
      `${rootPath}/package.json:/package.json`,
      `${rootPath}/package-lock.json:/package-lock.json`,
    ],
  },
};
DashDriveOptions.setDefaultCustomOptions(options);

beforeEach(function beforeEach() {
  if (!this.sinon) {
    this.sinon = sinon.sandbox.create();
  } else {
    this.sinon.restore();
  }
});

global.expect = expect;
